//! Parse engine observations and configured Android versions without side effects.
use crate::err;
use android_hub_domain::{Profile, Runtime};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub android_versions: BTreeMap<String, String>,
    pub profiles: Vec<Profile>,
    pub runtime: BTreeMap<String, Runtime>,
}
pub(super) fn parse_processes(
    text: &str,
    profiles: &[Profile],
) -> Result<BTreeMap<String, Runtime>, String> {
    let mut result = BTreeMap::new();
    for line in text.lines() {
        // Most host processes are unrelated. Avoid allocating their argument lists;
        // the exact executable/profile/port checks below remain authoritative.
        if !line.contains("qemu-system-aarch64") {
            continue;
        }
        let w: Vec<_> = line.split_whitespace().collect();
        if w.len() < 5 {
            continue;
        }
        let command_start = line.find(w[3]).ok_or("Thiếu command")?;
        let command = &line[command_start..];
        let executable = command.split(" -").next().unwrap_or(command);
        let engine = Path::new(executable)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if !["qemu-system-aarch64", "qemu-system-aarch64-headless"].contains(&engine) {
            continue;
        }
        let Some(avd) = w.windows(2).find(|p| p[0] == "-avd").map(|p| p[1]) else {
            continue;
        };
        let Some(profile) = profiles.iter().find(|p| p.id == avd) else {
            continue;
        };
        let port = profile.port.to_string();
        if !w.windows(2).any(|p| p[0] == "-port" && p[1] == port) {
            return Err(format!(
                "{} chạy ở port khác; không quản lý tự động",
                profile.name
            ));
        }
        let pid = w[0].parse::<u32>().map_err(err)?;
        let cpu = w[1].parse::<f32>().map_err(err)?;
        let rss = w[2].parse::<u64>().map_err(err)?;
        if !cpu.is_finite() || cpu < 0.0 {
            return Err("Số liệu CPU không hợp lệ".into());
        }
        if result
            .insert(
                avd.into(),
                Runtime {
                    pid: Some(pid),
                    cpu_percent: Some(cpu),
                    rss_kib: Some(rss),
                },
            )
            .is_some()
        {
            return Err("Có nhiều engine cùng tên máy".into());
        }
    }
    Ok(result)
}
pub(super) fn android_image_version(config: &str) -> Option<String> {
    let target = config
        .lines()
        .filter_map(|line| line.split_once('='))
        .find(|(key, _)| key.trim() == "target")?
        .1
        .trim();
    let api: u32 = target.strip_prefix("android-")?.parse().ok()?;
    let release = match api {
        31 => Some("12"),
        33 => Some("13"),
        34 => Some("14"),
        35 => Some("15"),
        36 => Some("16"),
        _ => None,
    };
    Some(match release {
        Some(release) => format!("Android {release} · API {api}"),
        None => format!("Android · API {api}"),
    })
}
#[cfg(test)]
mod version_tests {
    #[test]
    fn image_version_does_not_guess_unknown_or_missing_targets() {
        assert_eq!(
            super::android_image_version("target=android-36\n").as_deref(),
            Some("Android 16 · API 36")
        );
        assert_eq!(
            super::android_image_version("target=android-99").as_deref(),
            Some("Android · API 99")
        );
        assert_eq!(super::android_image_version("target=preview"), None);
        assert_eq!(super::android_image_version(""), None);
    }
}
