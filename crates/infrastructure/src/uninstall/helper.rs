//! One-shot copied executable; authorization is passed through a private pipe.
use super::*;
use crate::{SEQUENCE, file_lock::FileLock};
use std::{
    io::{BufRead, BufReader, Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    process::Stdio,
    sync::{atomic::Ordering, mpsc},
    thread,
};

#[derive(Serialize, Deserialize)]
struct Request {
    plan: UninstallPlan,
    keep: bool,
    parent: u32,
}

pub(super) fn launch(plan: UninstallPlan, keep: bool) -> Result<(), String> {
    let data_root = plan.root.clone();
    let work = std::env::temp_dir().join(format!(
        "android-hub-uninstall-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&work)
        .map_err(err)?;
    let result = (|| {
        let exe = work.join("android-hub-uninstaller");
        fs::copy(std::env::current_exe().map_err(err)?, &exe).map_err(err)?;
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o700)).map_err(err)?;
        let mut child = Command::new(&exe)
            .arg("--internal-uninstall")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(err)?;
        let send = serde_json::to_writer(
            child.stdin.take().ok_or("Thiếu kênh helper")?,
            &Request {
                plan,
                keep,
                parent: std::process::id(),
            },
        );
        if let Err(error) = send {
            let _ = child.kill();
            let _ = child.wait();
            return Err(err(error));
        }
        let stdout = child.stdout.take().ok_or("Thiếu phản hồi helper")?;
        let (tx, rx) = mpsc::channel();
        let reader = thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout)
                .take(4096)
                .read_line(&mut line)
                .map(|_| line);
            let _ = tx.send(result);
        });
        match rx.recv_timeout(Duration::from_secs(45)) {
            Ok(Ok(line)) if line.trim() == "READY" => {
                // Reap after the GUI exits if possible; the helper itself owns cleanup.
                thread::spawn(move || {
                    let _ = child.wait();
                });
                let _ = reader.join();
                Ok(())
            }
            response => {
                let helper_pid = child.id();
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                clear_owned_marker(&data_root, helper_pid);
                let detail = match response {
                    Ok(Ok(line)) => line,
                    _ => "Helper không phản hồi".into(),
                };
                Err(format!("Không khởi động được bộ gỡ cài đặt: {detail}"))
            }
        }
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&work);
    }
    result
}

pub fn run_uninstall_helper() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(err)?;
    let work = exe.parent().ok_or("Invalid helper path")?;
    let temp = std::env::temp_dir().canonicalize().map_err(err)?;
    if !work.canonicalize().map_err(err)?.starts_with(&temp)
        || !work
            .file_name()
            .is_some_and(|s| s.to_string_lossy().starts_with("android-hub-uninstall-"))
        || exe
            .file_name()
            .is_none_or(|s| s != "android-hub-uninstaller")
    {
        return Err("Invalid uninstall helper location".into());
    }
    let request: Request =
        serde_json::from_reader(std::io::stdin().take(1024 * 1024)).map_err(err)?;
    // SAFETY: getppid has no pointer arguments. The launching app must be this helper's parent.
    if request.parent != unsafe { libc::getppid() } as u32 {
        return Err("Uninstall parent changed".into());
    }
    let plan = &request.plan;
    let mut ready = false;
    let result = execute(&request, || {
        ready = true;
        println!("READY");
        std::io::stdout().flush().map_err(err)?;
        let deadline = Instant::now() + Duration::from_secs(60);
        while unsafe { libc::getppid() } as u32 == request.parent {
            if Instant::now() >= deadline {
                return Err("Hub chưa thoát; chưa xóa dữ liệu".into());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    });
    if let Err(error) = &result {
        if !ready {
            println!("ERROR: {}", error.replace('\n', " "));
            let _ = std::io::stdout().flush();
            let _ = fs::remove_dir_all(work);
            return result;
        }
        // Report failures even after the GUI has exited; no automatic retry.
        let report = work.join("Uninstall-error.txt");
        let text = format!(
            "Android Hub — uninstall incomplete / Gỡ cài đặt chưa hoàn tất\n\n{error}\n\nData: {}\nApp: {}\n\nSome confirmed data may already have been removed. Do not delete unrelated SDKs or folders.\nMột phần dữ liệu đã xác nhận có thể đã bị xóa. Không xóa SDK hoặc thư mục khác.\n",
            plan.root.display(),
            plan.app.path.display()
        );
        let _ = fs::write(&report, text);
        let _ = Command::new("/usr/bin/open")
            .arg("-t")
            .arg(&report)
            .status();
        // Keep only the diagnostic report, never a runnable orphan helper.
        let _ = fs::remove_file(&exe);
    } else {
        fs::remove_dir_all(work).map_err(err)?;
    }
    result
}

fn execute(
    request: &Request,
    wait_for_exit: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    execute_checked(request, wait_for_exit, || {
        ensure_no_other_processes(request.parent, std::process::id())?;
        let backend = Backend {
            root: request.plan.root.clone(),
            sdk: request.plan.sdk.clone(),
        };
        for p in backend.read_profiles()? {
            backend.ensure_stopped(&p)?;
        }
        Ok(())
    })
}

fn execute_checked(
    request: &Request,
    wait_for_exit: impl FnOnce() -> Result<(), String>,
    stopped: impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    let plan = &request.plan;
    plan.validate(request.keep)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(plan.root.join("hub.lock"))
        .map_err(err)?;
    let _guard = FileLock::acquire(lock, "Một thao tác Hub khác đang thực hiện")?;
    stopped()?;
    let marker = plan.root.join(PENDING);
    let mut pending = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&marker)
        .map_err(err)?;
    let result = (|| {
        serde_json::to_writer(
            &mut pending,
            &serde_json::json!({"helper_pid":std::process::id(),"parent_pid":request.parent}),
        )
        .map_err(err)?;
        pending.sync_all().map_err(err)?;
        wait_for_exit()?;
        stopped()?;
        plan.remove_data(request.keep)?;
        // Do not remove the bundle until every confirmed data entry was processed.
        fs::remove_dir_all(&plan.app.path).map_err(err)?;
        fs::remove_file(&marker).map_err(err)?;
        fs::remove_file(plan.root.join("hub.lock")).map_err(err)?;
        if !request.keep || !plan.backups.exists() {
            fs::remove_dir(&plan.root).map_err(err)?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&marker);
    }
    result
}

fn clear_owned_marker(root: &Path, helper_pid: u32) {
    // Only after killing/reaping our child, and only under the same lock.
    let Ok(file) = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join("hub.lock"))
    else {
        return;
    };
    let Ok(_guard) = FileLock::acquire(file, "busy") else {
        return;
    };
    let marker = root.join(PENDING);
    let Ok(bytes) = fs::read(&marker) else {
        return;
    };
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && value["helper_pid"] == helper_pid
        && value["parent_pid"] == std::process::id()
    {
        let _ = fs::remove_file(marker);
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::Fixture;
    use super::*;
    #[test]
    fn after_exit_cleanup_removes_only_confirmed_paths_and_preserves_backups_when_requested() {
        for keep in [false, true] {
            let f = Fixture::new();
            let request = Request {
                plan: f.plan(),
                keep,
                parent: 42,
            };
            execute_checked(
                &request,
                || {
                    assert!(f.backend.root.join("managed-sdks/sdk/image").exists());
                    assert!(f.app.exists());
                    assert!(f.backend.lock_raw().is_err());
                    Ok(())
                },
                || Ok(()),
            )
            .unwrap();
            assert!(!f.app.exists());
            assert_eq!(f.backend.root.exists(), keep);
            if keep {
                assert_eq!(fs::read_dir(&f.backend.root).unwrap().count(), 1);
            }
            assert!(f.backend.sdk.join("keep").exists());
        }
    }
    #[test]
    fn timeout_busy_lock_and_failed_shutdown_leave_app_and_data_intact() {
        let f = Fixture::new();
        let request = Request {
            plan: f.plan(),
            keep: false,
            parent: 42,
        };
        let lock = f.backend.lock_raw().unwrap();
        assert!(execute_checked(&request, || Ok(()), || Ok(())).is_err());
        drop(lock);
        assert!(execute_checked(&request, || Err("parent alive".into()), || Ok(())).is_err());
        assert!(execute_checked(&request, || Ok(()), || Err("engine running".into())).is_err());
        let checks = std::cell::Cell::new(0);
        assert!(
            execute_checked(
                &request,
                || Ok(()),
                || {
                    checks.set(checks.get() + 1);
                    if checks.get() == 2 {
                        Err("engine started during handoff".into())
                    } else {
                        Ok(())
                    }
                }
            )
            .is_err()
        );
        assert_eq!(checks.get(), 2);
        assert!(!f.backend.root.join(PENDING).exists());
        assert!(f.app.exists());
        assert!(f.backend.root.join("managed-sdks/sdk/image").exists());
        assert!(f.backend.root.join("backups/keep.txt").exists());
    }
    #[test]
    fn changed_scope_after_wait_is_rejected_and_unrelated_marker_is_not_cleared() {
        let f = Fixture::new();
        let request = Request {
            plan: f.plan(),
            keep: false,
            parent: 42,
        };
        assert!(
            execute_checked(
                &request,
                || {
                    fs::write(f.backend.root.join("new-data"), b"keep").unwrap();
                    Ok(())
                },
                || Ok(())
            )
            .is_err()
        );
        assert!(f.app.exists());
        assert!(f.backend.root.join("new-data").exists());
        fs::write(
            f.backend.root.join(PENDING),
            br#"{"helper_pid":7,"parent_pid":8}"#,
        )
        .unwrap();
        clear_owned_marker(&f.backend.root, 7);
        assert!(f.backend.root.join(PENDING).exists());
    }
}
