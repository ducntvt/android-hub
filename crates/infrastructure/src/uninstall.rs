//! Explicit self-uninstall. Never use SDK selection or arbitrary overrides as deletion roots.
mod helper;
#[cfg(test)]
mod tests;
use crate::{Backend, err};
pub use helper::run_uninstall_helper;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

pub(crate) const PENDING: &str = "uninstall-pending.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Identity {
    device: u64,
    inode: u64,
}
fn identity(path: &Path) -> Result<Identity, String> {
    let m = fs::symlink_metadata(path).map_err(err)?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err("Đường dẫn gỡ cài đặt không phải thư mục thật".into());
    }
    Ok(Identity {
        device: m.dev(),
        inode: m.ino(),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UninstallEntry {
    pub path: PathBuf,
    pub allocated_bytes: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UninstallPlan {
    pub app: UninstallEntry,
    pub data: Vec<UninstallEntry>,
    pub backups: PathBuf,
    root: PathBuf,
    sdk: PathBuf,
    root_identity: Identity,
    app_identity: Identity,
    entries: Vec<(PathBuf, u64, u64)>,
}
impl UninstallPlan {
    pub fn bytes_to_remove(&self, keep_backups: bool) -> u64 {
        self.data
            .iter()
            .filter(|e| !keep_backups || e.path != self.backups)
            .fold(self.app.allocated_bytes, |sum, e| {
                sum.saturating_add(e.allocated_bytes)
            })
    }
}

fn writable(path: &Path) -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    let path_c = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(err)?;
    // SAFETY: the C path is NUL-terminated and remains alive for access().
    if unsafe { libc::access(path_c.as_ptr(), libc::W_OK | libc::X_OK) } != 0 {
        return Err(format!("Không có quyền gỡ cài đặt tại: {}", path.display()));
    }
    Ok(())
}

fn tree_size(path: &Path, writable_tree: bool) -> Result<u64, String> {
    let device = fs::symlink_metadata(path).map_err(err)?.dev();
    let mut todo = vec![path.to_path_buf()];
    let mut seen = BTreeSet::new();
    let mut bytes = 0u64;
    let mut count = 0;
    let start = Instant::now();
    while let Some(path) = todo.pop() {
        count += 1;
        if count > 200_000 || start.elapsed() > Duration::from_secs(15) {
            return Err("Quét dữ liệu gỡ cài đặt vượt giới hạn; chưa xóa".into());
        }
        let meta = fs::symlink_metadata(&path).map_err(err)?;
        if meta.dev() != device || !(meta.is_file() || meta.is_dir() || meta.is_symlink()) {
            return Err("Dữ liệu chứa mount hoặc loại file không hỗ trợ; chưa xóa".into());
        }
        if seen.insert((meta.dev(), meta.ino())) {
            bytes = bytes.saturating_add(meta.blocks().saturating_mul(512));
        }
        if meta.is_dir() {
            if writable_tree {
                writable(&path)?;
            }
            for entry in fs::read_dir(path).map_err(err)? {
                if todo.len() + count >= 200_000 {
                    return Err("Quét dữ liệu gỡ cài đặt vượt giới hạn; chưa xóa".into());
                }
                todo.push(entry.map_err(err)?.path());
            }
        }
    }
    Ok(bytes)
}

fn bundle_id(app: &Path) -> Result<String, String> {
    let output = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleIdentifier"])
        .arg(app.join("Contents/Info.plist"))
        .output()
        .map_err(err)?;
    if !output.status.success() {
        return Err("Không xác minh được ứng dụng Android Hub".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().into())
}

fn validate_scope(app: &Path, root: &Path, home: &Path) -> Result<(), String> {
    // Reject aliases/symlinks in owned paths, including their ancestors.
    if app.canonicalize().map_err(err)? != app || root.canonicalize().map_err(err)? != root {
        return Err("Đường dẫn gỡ cài đặt chứa liên kết; không xóa".into());
    }
    let home = home.canonicalize().map_err(err)?;
    let standard = root == home.join("Library/Application Support/Android Hub")
        && (app == Path::new("/Applications/Android Hub.app")
            || app == home.join("Applications/Android Hub.app"))
        && bundle_id(app)? == "local.androidhub.desktop";
    let temp = std::env::temp_dir().canonicalize().map_err(err)?;
    let qa_parent = app.parent().ok_or("Thiếu thư mục app")?;
    let qa = app.file_name().is_some_and(|s| s == "Android Hub QA.app")
        && root == qa_parent.join("data")
        && (qa_parent.starts_with(&temp) || qa_parent.starts_with("/private/tmp"))
        && bundle_id(app)?.starts_with("local.androidhub.qa");
    if !standard && !qa {
        return Err(
            "Chỉ gỡ bản Hub trong Applications với dữ liệu mặc định, hoặc bản QA riêng".into(),
        );
    }
    identity(app)?;
    identity(root)?;
    Ok(())
}

fn entries(root: &Path) -> Result<Vec<(PathBuf, u64, u64)>, String> {
    let device = identity(root)?.device;
    let mut entries = Vec::new();
    for entry in fs::read_dir(root).map_err(err)? {
        let path = entry.map_err(err)?.path();
        if path.file_name().is_some_and(|n| n == PENDING) {
            continue;
        }
        let meta = fs::symlink_metadata(&path).map_err(err)?;
        if meta.dev() != device {
            return Err("Dữ liệu chứa mount; chưa xóa".into());
        }
        entries.push((path, meta.dev(), meta.ino()));
    }
    entries.sort();
    Ok(entries)
}

fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or("HOME không có".into())
}

impl Backend {
    pub fn prepare_uninstall(&self) -> Result<UninstallPlan, String> {
        let exe = std::env::current_exe().map_err(err)?;
        let app = exe
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .ok_or("Không tìm thấy app bundle")?;
        if exe != app.join("Contents/MacOS/android-hub") {
            return Err("Không xác minh được ứng dụng đang chạy".into());
        }
        self.uninstall_plan_at(app)
    }
    fn uninstall_plan_at(&self, app: &Path) -> Result<UninstallPlan, String> {
        validate_scope(app, &self.root, &home()?)?;
        if let Ok(meta) = fs::symlink_metadata(self.root.join("hub.lock"))
            && (!meta.is_file() || meta.file_type().is_symlink() || meta.nlink() != 1)
        {
            return Err("File khóa Hub không hợp lệ; không gỡ cài đặt".into());
        }
        let _guard = self.lock_raw()?;
        let entries = entries(&self.root)?;
        let mut data = Vec::new();
        for (path, _, _) in &entries {
            data.push(UninstallEntry {
                path: path.clone(),
                allocated_bytes: tree_size(path, false)?,
            });
        }
        Ok(UninstallPlan {
            app: UninstallEntry {
                path: app.into(),
                allocated_bytes: tree_size(app, false)?,
            },
            data,
            backups: self.root.join("backups"),
            root: self.root.clone(),
            sdk: self.sdk.clone(),
            root_identity: identity(&self.root)?,
            app_identity: identity(app)?,
            entries,
        })
    }
    pub fn launch_uninstall(&self, plan: UninstallPlan, keep_backups: bool) -> Result<(), String> {
        if std::env::current_exe().map_err(err)? != plan.app.path.join("Contents/MacOS/android-hub")
        {
            return Err("Không xác minh được ứng dụng đang chạy".into());
        }
        if self.root != plan.root || self.sdk != plan.sdk {
            return Err("Cấu hình đã đổi; xác nhận lại".into());
        }
        plan.validate(keep_backups)?;
        // The caller has finished normal shutdown. Recheck; never kill an unknown process.
        ensure_no_other_processes(std::process::id(), 0)?;
        for profile in self.read_profiles()? {
            self.ensure_stopped(&profile)?;
        }
        helper::launch(plan, keep_backups)
    }
}
impl UninstallPlan {
    fn validate(&self, keep: bool) -> Result<(), String> {
        validate_scope(&self.app.path, &self.root, &home()?)?;
        if identity(&self.root)? != self.root_identity
            || identity(&self.app.path)? != self.app_identity
            || entries(&self.root)? != self.entries
        {
            return Err("Dữ liệu gỡ cài đặt đã thay đổi; đóng hộp thoại và xác nhận lại".into());
        }
        writable(self.root.parent().ok_or("Thiếu thư mục dữ liệu")?)?;
        writable(self.app.path.parent().ok_or("Thiếu thư mục app")?)?;
        writable(&self.root)?;
        tree_size(&self.app.path, true)?;
        for (path, _, _) in &self.entries {
            if keep && *path == self.backups {
                identity(path)?;
            } else {
                tree_size(path, true)?;
            }
        }
        Ok(())
    }
    fn remove_data(&self, keep: bool) -> Result<(), String> {
        self.validate(keep)?;
        for (path, _, _) in &self.entries {
            if keep && *path == self.backups {
                continue;
            }
            // Keep the lock pathname until cleanup is complete.
            if path.file_name().is_some_and(|s| s == "hub.lock") {
                continue;
            }
            let m = fs::symlink_metadata(path).map_err(err)?;
            if m.is_dir() {
                fs::remove_dir_all(path).map_err(err)?;
            } else {
                fs::remove_file(path).map_err(err)?;
            }
        }
        Ok(())
    }
}

fn ensure_no_other_processes(parent: u32, helper: u32) -> Result<(), String> {
    let output = Command::new("/bin/ps")
        .args(["-axo", "pid=,comm="])
        .output()
        .map_err(err)?;
    if !output.status.success() {
        return Err("Không kiểm tra được tiến trình trước khi gỡ".into());
    }
    check_processes(&String::from_utf8_lossy(&output.stdout), parent, helper)
}
fn check_processes(text: &str, parent: u32, helper: u32) -> Result<(), String> {
    for line in text.lines() {
        let line = line.trim();
        let Some((pid, command)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let pid = pid.parse::<u32>().map_err(err)?;
        if pid == parent || pid == helper {
            continue;
        }
        let name = Path::new(command.trim())
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if [
            "android-hub",
            "android-hub-uninstaller",
            "qemu-system-aarch64",
            "qemu-system-aarch64-headless",
        ]
        .contains(&name)
        {
            return Err("Còn Hub hoặc máy Android khác đang chạy; đóng chúng trước khi gỡ".into());
        }
    }
    Ok(())
}
