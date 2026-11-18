//! Read-only macOS engine RSS/%CPU sampler. Output is JSONL, never UI/account data.
use std::{
    error::Error,
    fs::OpenOptions,
    io::Write,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn ps(pid: u32, fields: &[&str]) -> Result<String, Box<dyn Error>> {
    let mut cmd = Command::new("/bin/ps");
    cmd.env("LC_ALL", "C").args(["-p", &pid.to_string()]);
    for field in fields {
        cmd.args(["-o", field]);
    }
    let output = cmd.output()?;
    if !output.status.success() {
        return Err("process unavailable or ps access denied".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn values(text: &str) -> Result<(u64, f64), Box<dyn Error>> {
    let words: Vec<_> = text.split_whitespace().collect();
    if words.len() != 2 {
        return Err("unexpected ps metric columns".into());
    }
    let rss = words[0].parse()?;
    let cpu: f64 = words[1].parse()?;
    if !cpu.is_finite() || cpu < 0.0 {
        return Err("invalid CPU value".into());
    }
    Ok((rss, cpu))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: engine-metrics PID SAMPLE_COUNT INTERVAL_SECONDS NEW_OUTPUT.jsonl".into(),
        );
    }
    let pid: u32 = args[1].parse()?;
    let count: u32 = args[2].parse()?;
    let interval: u64 = args[3].parse()?;
    if pid == 0 || !(1..=10000).contains(&count) || !(1..=60).contains(&interval) {
        return Err("PID must be positive, count 1..10000, interval 1..60 seconds".into());
    }
    let executable = ps(pid, &["comm="])?;
    let name = Path::new(&executable)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid executable name")?;
    if !["qemu-system-aarch64", "qemu-system-aarch64-headless"].contains(&name) {
        return Err("PID is not an allowed Android Emulator executable".into());
    }
    let identity = ps(pid, &["lstart=", "comm="])?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[4])?;
    writeln!(
        file,
        "{{\"event\":\"start\",\"schema\":1,\"pid\":{pid},\"engine\":\"{name}\",\"samples_requested\":{count},\"interval_seconds\":{interval},\"rss_unit\":\"KiB\",\"cpu_source\":\"macOS ps %cpu estimate; one CPU = 100 percent\",\"scope\":\"single process only; excludes helpers and GPU allocations; not FPS or gameplay evidence\"}}"
    )?;
    let started = Instant::now();
    for index in 0..count {
        if index > 0 {
            let target = Duration::from_secs(interval * u64::from(index));
            if let Some(wait) = target.checked_sub(started.elapsed()) {
                thread::sleep(wait);
            }
        }
        let sample = (|| -> Result<(u64, f64), Box<dyn Error>> {
            if ps(pid, &["lstart=", "comm="])? != identity {
                return Err("process identity changed".into());
            }
            let result = values(&ps(pid, &["rss=", "%cpu="])?)?;
            if ps(pid, &["lstart=", "comm="])? != identity {
                return Err("process identity changed during sample".into());
            }
            Ok(result)
        })();
        let (rss, cpu) = match sample {
            Ok(value) => value,
            Err(error) => {
                writeln!(
                    file,
                    "{{\"event\":\"end\",\"complete\":false,\"samples_written\":{index},\"reason\":\"process unavailable, changed, or metric parse failed\"}}"
                )?;
                file.flush()?;
                return Err(error);
            }
        };
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64();
        writeln!(
            file,
            "{{\"event\":\"sample\",\"index\":{index},\"unix_seconds\":{timestamp},\"elapsed_seconds\":{},\"rss_kib\":{rss},\"cpu_percent_ps\":{cpu}}}",
            started.elapsed().as_secs_f64()
        )?;
        file.flush()?;
    }
    writeln!(
        file,
        "{{\"event\":\"end\",\"complete\":true,\"samples_written\":{count}}}"
    )?;
    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::values;

    #[test]
    fn parses_multicore_cpu_without_clamping() {
        assert_eq!(values("  1048576  213.7\n").unwrap(), (1048576, 213.7));
    }

    #[test]
    fn rejects_missing_extra_or_nonfinite_metrics() {
        for input in ["", "123", "123 4 5", "-1 0", "1 NaN", "1 inf", "1 -2"] {
            assert!(values(input).is_err(), "{input}");
        }
    }
}
