//! Explicit, guest-origin connectivity checks. No settings repair or periodic polling.
use crate::{Backend, scroll::boot_identity, scroll::transport_identity};
use android_hub_domain::Profile;
use sha2::{Digest, Sha256};
use std::{
    process::Stdio,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

const DEX: &[u8] = include_bytes!("../android/connectivity-probe.dex");
pub const CONNECTIVITY_ENDPOINT: &str = "https://www.google.com/generate_204";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeResult {
    Passed,
    NoAddress,
    DnsFailed,
    TlsFailed,
    TimedOut,
    IoFailed,
    HttpStatus(u16),
    Unavailable,
}
impl ProbeResult {
    pub fn code(&self) -> String {
        match self {
            Self::Passed => "OK".into(),
            Self::NoAddress => "NO_ADDRESS".into(),
            Self::DnsFailed => "DNS".into(),
            Self::TlsFailed => "TLS".into(),
            Self::TimedOut => "TIMEOUT".into(),
            Self::IoFailed => "IO".into(),
            Self::Unavailable => "UNAVAILABLE".into(),
            Self::HttpStatus(status) => format!("HTTP_{status}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectivityReport {
    pub measured_seconds: u64,
    pub boot_id: String,
    pub interface: ProbeResult,
    pub dns: ProbeResult,
    pub https: ProbeResult,
}
impl ConnectivityReport {
    /// Copyable technical details contain only fixed labels, endpoint and codes.
    /// Do not include guest addresses, adb output, cookies, headers or identifiers.
    pub fn details(&self) -> String {
        format!(
            "Android Hub connectivity v1\nUTC epoch: {}\nGuest shell UID; endpoint: {}\nINTERFACE {}\nDNS {}\nHTTPS {}\nA passed endpoint check does not verify other apps or services.",
            self.measured_seconds,
            CONNECTIVITY_ENDPOINT,
            self.interface.code(),
            self.dns.code(),
            self.https.code()
        )
    }
}

fn parse_result(value: &str, step: &str) -> Result<ProbeResult, String> {
    let result = match value {
        "OK" => ProbeResult::Passed,
        "TIMEOUT" => ProbeResult::TimedOut,
        "IO" => ProbeResult::IoFailed,
        "UNAVAILABLE" => ProbeResult::Unavailable,
        "NO_ADDRESS" if step == "INTERFACE" => ProbeResult::NoAddress,
        "DNS" if step != "INTERFACE" => ProbeResult::DnsFailed,
        "TLS" if step == "HTTPS" => ProbeResult::TlsFailed,
        value if step == "HTTPS" && value.starts_with("HTTP_") => {
            let status = value[5..]
                .parse::<u16>()
                .ok()
                .filter(|s| (100..=599).contains(s));
            match status {
                Some(status) if value == format!("HTTP_{status}") && status != 204 => {
                    ProbeResult::HttpStatus(status)
                }
                _ => return Err("Invalid connectivity response".into()),
            }
        }
        _ => return Err("Invalid connectivity response".into()),
    };
    Ok(result)
}

fn parse_report(value: &str, expected_boot: &str) -> Result<ConnectivityReport, String> {
    let lines: Vec<_> = value.lines().collect();
    if lines.len() != 5 || lines[0] != format!("HUBNET1 {expected_boot}") || lines[4] != "DONE" {
        return Err("Invalid connectivity response or changed Android session".into());
    }
    let step = |index: usize, name: &str| {
        let value = lines[index]
            .strip_prefix(&format!("{name} "))
            .ok_or("Invalid connectivity response")?;
        parse_result(value, name)
    };
    Ok(ConnectivityReport {
        measured_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Host clock unavailable")?
            .as_secs(),
        boot_id: expected_boot.to_owned(),
        interface: step(1, "INTERFACE")?,
        dns: step(2, "DNS")?,
        https: step(3, "HTTPS")?,
    })
}

fn launch_script(boot: &str, token: &str) -> String {
    let hash: String = Sha256::digest(DEX)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    // Every interpolated value is generated or validated, never user text.
    // dd consumes exactly the DEX byte count; stdin stays open for the helper's
    // disconnect monitor. One transport owns upload, execution and cleanup.
    format!(
        "[ \"$(cat /proc/sys/kernel/random/boot_id)\" = '{boot}' ] || exit 3; \
         d=/data/local/tmp/android-hub-connectivity-{token}; umask 077; mkdir \"$d\" || exit 6; \
         trap 'rm -f \"$d/probe.dex\"; rmdir \"$d\"' EXIT; \
         dd bs=1 count={} of=\"$d/probe.dex\" 2>/dev/null || exit 7; \
         set -- $(sha256sum \"$d/probe.dex\"); [ \"$1\" = '{hash}' ] || exit 8; \
         chmod 444 \"$d/probe.dex\" || exit 9; \
         CLASSPATH=\"$d/probe.dex\" app_process / local.androidhub.ConnectivityProbe '{boot}'",
        DEX.len()
    )
}

async fn invoke(
    mut command: Command,
    payload: &[u8],
    boot: &str,
) -> Result<ConnectivityReport, String> {
    tokio::time::timeout(Duration::from_secs(25), async {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| "Connectivity helper could not start")?;
        let mut input = child.stdin.take().ok_or("Connectivity input unavailable")?;
        input
            .write_all(payload)
            .await
            .map_err(|_| "Connectivity helper upload failed")?;
        input
            .flush()
            .await
            .map_err(|_| "Connectivity helper upload failed")?;
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or("Connectivity output unavailable")?
            .take(2049)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| "Connectivity response unavailable")?;
        if bytes.len() > 2048 {
            return Err("Connectivity response exceeded limit".into());
        }
        let status = child
            .wait()
            .await
            .map_err(|_| "Connectivity helper unavailable")?;
        // Hold stdin until exit; dropping it earlier would cancel the guest probe.
        drop(input);
        if !status.success() {
            return Err("Connectivity helper unavailable or interrupted".into());
        }
        let value = std::str::from_utf8(&bytes).map_err(|_| "Invalid connectivity response")?;
        parse_report(value, boot)
    })
    .await
    .map_err(|_| "Connectivity helper timed out".to_owned())?
}

impl Backend {
    /// A cancelled future kills the adb child; guest EOF/watchdog terminates its
    /// helper. No global mutation lock is held, so display input remains active.
    pub async fn check_connectivity(
        &self,
        profile: &Profile,
        pid: u32,
        expected_boot: &str,
    ) -> Result<ConnectivityReport, String> {
        let boot = boot_identity(expected_boot).map_err(|_| "Android session unavailable")?;
        let before = self
            .observe_boot(profile, pid)
            .await
            .map_err(|_| "Android session could not be verified")?;
        if !before.completed || before.boot_id != boot {
            return Err("Android session changed or is not ready".into());
        }
        let adb = self.sdk.join("platform-tools/adb");
        let mut list = Command::new(&adb);
        list.args(["devices", "-l"]);
        let devices = crate::lifecycle::output(list)
            .await
            .map_err(|_| "ADB transport unavailable")?;
        let transport = transport_identity(&devices, &format!("emulator-{}", profile.port))
            .map_err(|_| "ADB transport unavailable")?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Host clock unavailable")?
            .as_nanos();
        let sequence = crate::SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let script = launch_script(boot, &format!("{}-{stamp}-{sequence}", std::process::id()));
        let mut command = Command::new(adb);
        command.args(["-t", &transport, "shell", "-T", &script]);
        let report = invoke(command, DEX, boot).await?;
        let after = self
            .observe_boot(profile, pid)
            .await
            .map_err(|_| "Android session could not be reverified")?;
        if !after.completed || after.boot_id != boot {
            return Err("Android session changed; result discarded".into());
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const BOOT: &str = "12345678-1234-1234-1234-123456789abc";
    fn report(interface: &str, dns: &str, https: &str) -> String {
        format!("HUBNET1 {BOOT}\nINTERFACE {interface}\nDNS {dns}\nHTTPS {https}\nDONE\n")
    }
    #[test]
    fn protocol_distinguishes_failure_from_unavailable_and_rejects_private_output() {
        let parsed = parse_report(&report("NO_ADDRESS", "TIMEOUT", "TLS"), BOOT).unwrap();
        assert_eq!(parsed.interface, ProbeResult::NoAddress);
        assert_eq!(parsed.dns, ProbeResult::TimedOut);
        assert_eq!(parsed.https, ProbeResult::TlsFailed);
        assert_eq!(
            parse_report(&report("UNAVAILABLE", "DNS", "HTTP_400"), BOOT)
                .unwrap()
                .https,
            ProbeResult::HttpStatus(400)
        );
        for bad in [
            "HTTP_204",
            "HTTP_999",
            "HTTP_0200",
            "TLS private-host",
            "secret-token",
        ] {
            assert!(parse_report(&report("OK", "OK", bad), BOOT).is_err());
        }
        assert!(parse_report(&report("TLS", "OK", "OK"), BOOT).is_err());
        assert!(parse_report(&report("OK", "OK", "OK"), "different-session").is_err());
        assert!(parse_report(&(report("OK", "OK", "OK") + "private data"), BOOT).is_err());
        assert!(
            !parse_report(&report("OK", "OK", "OK"), BOOT)
                .unwrap()
                .details()
                .contains(BOOT)
        );
    }
    #[tokio::test]
    async fn missing_helper_is_unknown_not_a_network_failure() {
        assert!(
            invoke(Command::new("/nonexistent/android-hub-helper"), b"", BOOT)
                .await
                .is_err()
        );
        assert!(
            invoke(Command::new("/usr/bin/false"), b"", BOOT)
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn transport_holds_stdin_open_and_accepts_only_complete_report() {
        let value = report("OK", "OK", "OK");
        let mut command = Command::new("/bin/sh");
        command.args([
            "-c",
            "dd bs=1 count=4 of=/dev/null 2>/dev/null; printf '%s' \"$1\"",
            "fixture",
            &value,
        ]);
        let result = invoke(command, b"DEX!", BOOT).await.unwrap();
        assert_eq!(result.https, ProbeResult::Passed);
    }
    #[tokio::test]
    async fn cancellation_reaps_the_transport_child() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "hub-connectivity-cancel-{}-{stamp}",
            std::process::id()
        ));
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "echo $$ > \"$1\"; exec sleep 30", "fixture"])
            .arg(&path);
        assert!(
            tokio::time::timeout(Duration::from_millis(300), invoke(command, b"", BOOT))
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
        while unsafe { libc::kill(pid, 0) } == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "transport survived cancellation"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    #[tokio::test]
    async fn bounded_output_rejects_excess_without_waiting_for_eof() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "head -c 4096 /dev/zero; exec sleep 30"]);
        let result = tokio::time::timeout(Duration::from_secs(2), invoke(command, b"", BOOT)).await;
        assert!(result.unwrap().is_err());
    }
    #[test]
    fn helper_launch_keeps_upload_in_the_boot_checked_channel() {
        let script = launch_script(BOOT, "test-123");
        assert!(script.starts_with("[ \"$(cat /proc/sys/kernel/random/boot_id)\""));
        assert!(script.contains(&format!("dd bs=1 count={}", DEX.len())));
        assert!(script.contains("trap 'rm -f"));
        assert!(script.contains("chmod 444"));
        assert!(!script.contains("rm -rf"));
    }
}
