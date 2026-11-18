//! Bounded, read-only readiness observation. A frame or PID is not boot readiness.
use crate::{
    Backend, err, parse_processes,
    scroll::{boot_identity, transport_identity},
};
use android_hub_domain::Profile;
use std::{process::Stdio, time::Duration};
use tokio::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootObservation {
    pub boot_id: String,
    pub completed: bool,
}

pub(crate) async fn output(mut command: Command) -> Result<String, String> {
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        command
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output(),
    )
    .await
    .map_err(|_| "Kiểm tra Android quá thời gian".to_owned())?
    .map_err(err)?;
    if !result.status.success() || result.stdout.len() > 8192 {
        return Err("Chưa đọc được trạng thái Android".into());
    }
    String::from_utf8(result.stdout).map_err(err)
}

impl Backend {
    async fn probe_engine(&self, profile: &Profile, pid: u32) -> Result<(), String> {
        let mut command = Command::new("/bin/ps");
        command.args(["-p", &pid.to_string(), "-o", "pid=,%cpu=,rss=,command="]);
        let value = output(command).await?;
        if parse_processes(&value, std::slice::from_ref(profile))?
            .get(&profile.id)
            .and_then(|r| r.pid)
            != Some(pid)
        {
            return Err("Phiên Android đã thay đổi".into());
        }
        Ok(())
    }
    async fn boot_adb(&self, args: &[&str]) -> Result<String, String> {
        let mut command = Command::new(self.sdk.join("platform-tools/adb"));
        command.args(args);
        output(command).await
    }
    /// Dropping this future kills its current child. Never writes guest settings.
    pub async fn observe_boot(
        &self,
        profile: &Profile,
        pid: u32,
    ) -> Result<BootObservation, String> {
        tokio::time::timeout(Duration::from_secs(12), async {
            profile.validate()?;
            self.probe_engine(profile, pid).await?;
            let serial = format!("emulator-{}", profile.port);
            let devices = self.boot_adb(&["devices", "-l"]).await?;
            let transport = transport_identity(&devices, &serial)?;
            // `adb emu` selects its console using -s; -t is not supported by
            // this adb client path when multiple emulators exist. Shell reads
            // remain pinned to the transport, rechecked after the console query.
            let identity = self
                .boot_adb(&["-s", &serial, "emu", "avd", "name"])
                .await?;
            if identity.lines().next().map(str::trim) != Some(profile.id.as_str()) {
                return Err("ADB trả về máy khác; bỏ qua trạng thái boot".into());
            }
            let devices = self.boot_adb(&["devices", "-l"]).await?;
            if transport_identity(&devices, &serial)? != transport {
                return Err("Transport Android đã thay đổi".into());
            }
            // Read boot identity both before and after property evaluation so a
            // soft reboot cannot combine old readiness with a new guest session.
            let before = self
                .boot_adb(&[
                    "-t",
                    &transport,
                    "shell",
                    "cat /proc/sys/kernel/random/boot_id; getprop sys.boot_completed",
                ])
                .await?;
            let observation = parse_boot(&before)?;
            let after = self
                .boot_adb(&[
                    "-t",
                    &transport,
                    "shell",
                    "cat",
                    "/proc/sys/kernel/random/boot_id",
                ])
                .await?;
            if boot_identity(&after)? != observation.boot_id {
                return Err("Phiên boot Android đã thay đổi".into());
            }
            self.probe_engine(profile, pid).await?;
            Ok(observation)
        })
        .await
        .map_err(|_| "Kiểm tra Android quá thời gian".to_owned())?
    }
}

fn parse_boot(value: &str) -> Result<BootObservation, String> {
    let mut lines = value.lines();
    let boot_id = boot_identity(lines.next().unwrap_or_default())?.to_owned();
    let completed = match lines.next().unwrap_or_default().trim() {
        "1" => true,
        "" | "0" => false,
        _ => return Err("Trạng thái boot Android không hợp lệ".into()),
    };
    if lines.any(|line| !line.trim().is_empty()) {
        return Err("Phản hồi boot Android không hợp lệ".into());
    }
    Ok(BootObservation { boot_id, completed })
}

#[cfg(test)]
mod tests {
    use super::*;
    const BOOT: &str = "12345678-1234-1234-1234-123456789abc";
    #[test]
    fn readiness_requires_explicit_property_and_valid_boot_identity() {
        assert!(parse_boot(&format!("{BOOT}\n1\n")).unwrap().completed);
        assert!(!parse_boot(&format!("{BOOT}\n\n")).unwrap().completed);
        assert!(!parse_boot(&format!("{BOOT}\n0\n")).unwrap().completed);
        assert!(parse_boot("1\n").is_err());
        assert!(parse_boot(&format!("{BOOT}\ntrue\n")).is_err());
        assert!(parse_boot(&format!("{BOOT}\n1\nextra\n")).is_err());
    }
    #[tokio::test]
    async fn failed_read_is_unknown_instead_of_completed() {
        let command = Command::new("/usr/bin/false");
        assert!(output(command).await.is_err());
    }
    #[tokio::test]
    async fn canceling_a_probe_reaps_its_subprocess() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("hub-probe-cancel-{}-{stamp}", std::process::id()));
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "echo $$ > \"$1\"; exec /bin/sleep 30", "probe"])
            .arg(&path);
        assert!(
            tokio::time::timeout(Duration::from_millis(300), output(command))
                .await
                .is_err()
        );
        let pid: i32 = std::fs::read_to_string(&path)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        std::fs::remove_file(path).unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            // Signal 0 observes existence; it never sends a signal to the child.
            if unsafe { libc::kill(pid, 0) } != 0 {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "probe child survived cancellation"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}
