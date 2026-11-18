//! Conservative disk budget for offline copies; never deletes data to make room.
use crate::{backup, err};
use std::{fs, path::Path};
const RESERVE: u64 = 256 * 1024 * 1024;
// Initial admission policy for the pinned Pixel 5/API 36 profile: observed
// 10 GiB data partition, 512 MiB SD card, cache and metadata headroom.
// This is not a reservation or a bound on future guest disk growth.
pub(super) const CREATE_MINIMUM: u64 = 12 * 1024 * 1024 * 1024;

fn check_create_budget(available: u64, destination: &Path) -> Result<(), String> {
    if available < CREATE_MINIMUM {
        return Err(format!(
            "Chưa đủ dung lượng để tạo máy tại {}: Hub yêu cầu tối thiểu 12 GiB trống, hiện còn {:.2} GiB. Hãy giải phóng dung lượng rồi thử lại; SDK chưa được chạy để tạo máy.",
            destination.display(),
            available as f64 / 1073741824.
        ));
    }
    Ok(())
}
pub(super) fn ensure_creation_space(destination: &Path) -> Result<(), String> {
    let available = fs2::available_space(destination).map_err(|e| {
        format!(
            "Không đọc được dung lượng tại {}: {e}",
            destination.display()
        )
    })?;
    check_create_budget(available, destination)
}

fn required_space(sizes: impl IntoIterator<Item = u64>, block: u64) -> Result<u64, String> {
    if block == 0 {
        return Err("Filesystem báo kích thước block không hợp lệ".into());
    }
    sizes.into_iter().try_fold(RESERVE, |total, bytes| {
        let rounded = bytes
            .checked_add((block - bytes % block) % block)
            .ok_or("Kích thước bản sao vượt giới hạn")?;
        total
            .checked_add(rounded)
            .ok_or_else(|| "Kích thước bản sao vượt giới hạn".into())
    })
}
fn check_budget(required: u64, available: u64, destination: &Path) -> Result<(), String> {
    if available < required {
        return Err(format!(
            "Không đủ dung lượng tại {}: cần {:.2} GiB (gồm 256 MiB dự phòng), còn {:.2} GiB. Hãy giải phóng dung lượng rồi thử lại.",
            destination.display(),
            required as f64 / 1073741824.,
            available as f64 / 1073741824.
        ));
    }
    Ok(())
}
pub(super) fn ensure_copy_space(source: &Path, destination: &Path) -> Result<(), String> {
    let stats = fs2::statvfs(destination).map_err(|e| {
        format!(
            "Không đọc được dung lượng tại {}: {e}",
            destination.display()
        )
    })?;
    let sizes = backup::files(source)?
        .into_iter()
        .map(|relative| {
            fs::symlink_metadata(source.join(relative))
                .map_err(err)
                .and_then(|meta| {
                    if meta.is_file() {
                        Ok(meta.len())
                    } else {
                        Err("Dữ liệu nguồn thay đổi khi tính dung lượng".into())
                    }
                })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let required = required_space(sizes, stats.allocation_granularity())?;
    check_budget(required, stats.available_space(), destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creation_budget_checks_boundary_without_overflow_or_writes() {
        let destination = Path::new("/fixture/avds");
        assert!(check_create_budget(CREATE_MINIMUM, destination).is_ok());
        assert!(check_create_budget(u64::MAX, destination).is_ok());
        for free in [0, CREATE_MINIMUM - 1] {
            let error = check_create_budget(free, destination).unwrap_err();
            assert!(error.contains("12 GiB"));
            assert!(error.contains("/fixture/avds"));
            assert!(error.contains("SDK chưa được chạy"));
        }
        assert!(ensure_creation_space(Path::new("/nonexistent-hub-create-space")).is_err());
    }
    #[test]
    fn counts_logical_bytes_rounding_and_reserve_without_overflow() {
        assert_eq!(
            required_space([0, 1, 4096, 4097], 4096).unwrap(),
            RESERVE + 16384
        );
        assert_eq!(required_space([], 4096).unwrap(), RESERVE);
        assert!(required_space([u64::MAX], 4096).is_err());
        assert!(required_space([u64::MAX - RESERVE, 1], 1).is_err());
        assert!(required_space([1], 0).is_err());
    }
    #[test]
    fn insufficient_disk_reports_required_available_and_destination() {
        let path = Path::new("/test/backup");
        assert!(check_budget(RESERVE, RESERVE, path).is_ok());
        let error = check_budget(RESERVE, RESERVE - 1, path).unwrap_err();
        assert!(error.contains("/test/backup"));
        assert!(error.contains("cần 0.25 GiB"));
        assert!(error.contains("còn"));
        assert!(
            ensure_copy_space(
                Path::new("/nonexistent-hub-source"),
                Path::new("/nonexistent-hub-destination")
            )
            .is_err()
        );
    }
}
