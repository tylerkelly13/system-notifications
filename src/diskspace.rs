//! Disk space monitoring
//!
//! Monitors disk space usage and sends notifications when thresholds are crossed.

use crate::common::{notify, App, NotificationType};
use crate::config::DiskConfig;
use std::path::Path;
use sysinfo::Disks;

/// Default paths to monitor
pub const DEFAULT_PATHS: &[&str] = &["/", "/home"];

/// Disk space information for a single mount point
#[derive(Debug, Clone, PartialEq)]
pub struct DiskSpaceInfo {
    /// The mount path
    pub path: String,
    /// Total space in bytes
    pub total_bytes: u64,
    /// Used space in bytes
    pub used_bytes: u64,
    /// Available space in bytes
    pub available_bytes: u64,
    /// Percentage used (0.0 to 100.0)
    pub percent_used: f32,
}

/// Converts bytes to human-readable format.
///
/// # Arguments
///
/// * `bytes` - The number of bytes to format
///
/// # Returns
///
/// A string with the value and appropriate unit (B, K, M, G, T, P, E, Z, Y)
///
/// # Example
///
/// ```
/// # use system_notifier::diskspace::bytes_to_human;
/// assert_eq!(bytes_to_human(10000), "9.8K");
/// assert_eq!(bytes_to_human(100001221), "95.4M");
/// ```
pub fn bytes_to_human(bytes: u64) -> String {
    const SYMBOLS: &[char] = &['K', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y'];
    const UNITS: &[u64] = &[
        1u64 << 10, // K
        1u64 << 20, // M
        1u64 << 30, // G
        1u64 << 40, // T
        1u64 << 50, // P
        1u64 << 60, // E
    ];

    if bytes < 1024 {
        return format!("{}B", bytes);
    }

    for (i, unit) in UNITS.iter().enumerate().rev() {
        if bytes >= *unit {
            let value = bytes as f32 / *unit as f32;
            return format!("{:.1}{}", value, SYMBOLS[i]);
        }
    }

    format!("{}B", bytes)
}

/// Retrieves disk space information for a specific path.
///
/// # Arguments
///
/// * `path` - The filesystem path to check
///
/// # Returns
///
/// * `Some(DiskSpaceInfo)` if the path exists and information is available
/// * `None` if the path does not exist or information cannot be retrieved
pub fn get_disk_space_info(path: &str) -> Option<DiskSpaceInfo> {
    let disks = Disks::new_with_refreshed_list();

    let path_obj = Path::new(path);

    // Find the disk that contains this path
    let disk = disks
        .iter()
        .find(|d| path_obj.starts_with(d.mount_point()))?;

    let total_bytes = disk.total_space();
    let available_bytes = disk.available_space();
    let used_bytes = total_bytes.saturating_sub(available_bytes);

    let percent_used = if total_bytes > 0 {
        (used_bytes as f32 / total_bytes as f32) * 100.0
    } else {
        0.0
    };

    Some(DiskSpaceInfo {
        path: path.to_string(),
        total_bytes,
        used_bytes,
        available_bytes,
        percent_used,
    })
}

/// Formats the disk space notification message.
///
/// # Arguments
///
/// * `info` - The disk space information
///
/// # Returns
///
/// A formatted message string describing the disk usage
pub fn format_diskspace_message(info: &DiskSpaceInfo) -> String {
    format!(
        "{} is {:.1}% full, only {} of {} remains unused.",
        info.path,
        info.percent_used,
        bytes_to_human(info.available_bytes),
        bytes_to_human(info.total_bytes)
    )
}

/// Determines which notification should be sent based on disk usage and explicit thresholds.
fn determine_notification_with_thresholds(
    info: &DiskSpaceInfo,
    low: f32,
    very_low: f32,
    critical: f32,
) -> Option<(String, String, NotificationType)> {
    let percent = info.percent_used;
    let message = format_diskspace_message(info);

    if percent >= critical {
        Some(("Disk space critical".to_string(), message, NotificationType::Error))
    } else if percent >= very_low {
        Some(("Low disk space warning".to_string(), message, NotificationType::Info))
    } else if percent >= low {
        Some(("Low disk space notice".to_string(), message, NotificationType::Info))
    } else {
        None
    }
}

/// Checks disk space and sends notifications if needed.
///
/// Paths and thresholds are read from `config`. If `config.disabled` is `true`,
/// returns immediately without checking.
pub fn check_and_notify(config: &DiskConfig) {
    if config.disabled.unwrap_or(false) {
        return;
    }

    // After Config::load() deep-merges with defaults, all fields are
    // guaranteed to be Some. Panicking here would indicate a programming
    // error (check_and_notify called without a merged config).
    let paths = config.paths.as_deref().unwrap();
    let t = config.thresholds.as_ref().unwrap();
    let low      = t.low.unwrap();
    let very_low = t.very_low.unwrap();
    let critical = t.critical.unwrap();

    for path in paths {
        if let Some(info) = get_disk_space_info(path) {
            if let Some((title, message, notification_type)) =
                determine_notification_with_thresholds(&info, low, very_low, critical)
            {
                notify(notification_type, App::DiskSpace, &title, &message);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shorthand: call the internal helper with the configured default thresholds (90/95/97.5).
    fn notify_default(info: &DiskSpaceInfo) -> Option<(String, String, NotificationType)> {
        determine_notification_with_thresholds(info, 90.0, 95.0, 97.5)
    }

    fn disk_info(percent_used: f32) -> DiskSpaceInfo {
        let total = 100_000_000_000u64;
        let used = (total as f32 * percent_used / 100.0) as u64;
        DiskSpaceInfo {
            path: "/".to_string(),
            total_bytes: total,
            used_bytes: used,
            available_bytes: total.saturating_sub(used),
            percent_used,
        }
    }

    #[test]
    fn test_bytes_to_human_zero() {
        assert_eq!(bytes_to_human(0), "0B");
    }

    #[test]
    fn test_bytes_to_human_bytes() {
        assert_eq!(bytes_to_human(500), "500B");
    }

    #[test]
    fn test_bytes_to_human_kilobytes() {
        assert_eq!(bytes_to_human(10000), "9.8K");
    }

    #[test]
    fn test_bytes_to_human_megabytes() {
        assert_eq!(bytes_to_human(100001221), "95.4M");
    }

    #[test]
    fn test_bytes_to_human_gigabytes() {
        assert_eq!(bytes_to_human(5368709120), "5.0G");
    }

    #[test]
    fn test_bytes_to_human_exact_units() {
        assert_eq!(bytes_to_human(1024), "1.0K");
        assert_eq!(bytes_to_human(1048576), "1.0M");
        assert_eq!(bytes_to_human(1073741824), "1.0G");
    }

    #[test]
    fn test_format_diskspace_message() {
        let msg = format_diskspace_message(&disk_info(90.0));
        assert!(msg.contains("/"));
        assert!(msg.contains("90.0%"));
        assert!(msg.contains("full"));
    }

    #[test]
    fn test_determine_notification_ok() {
        assert_eq!(notify_default(&disk_info(50.0)), None);
    }

    #[test]
    fn test_determine_notification_low() {
        let (title, message, notif_type) = notify_default(&disk_info(91.0)).unwrap();
        assert_eq!(title, "Low disk space notice");
        assert!(message.contains("91.0%"));
        assert_eq!(notif_type, NotificationType::Info);
    }

    #[test]
    fn test_determine_notification_very_low() {
        let (title, message, notif_type) = notify_default(&disk_info(96.0)).unwrap();
        assert_eq!(title, "Low disk space warning");
        assert!(message.contains("96.0%"));
        assert_eq!(notif_type, NotificationType::Info);
    }

    #[test]
    fn test_determine_notification_critical() {
        let (title, message, notif_type) = notify_default(&disk_info(98.0)).unwrap();
        assert_eq!(title, "Disk space critical");
        assert!(message.contains("98.0%"));
        assert_eq!(notif_type, NotificationType::Error);
    }

    #[test]
    fn test_determine_notification_at_thresholds() {
        assert!(notify_default(&disk_info(90.0)).is_some());
        assert!(notify_default(&disk_info(95.0)).is_some());
        assert!(notify_default(&disk_info(97.5)).is_some());
    }

    #[test]
    fn test_determine_notification_custom_thresholds() {
        // With defaults, 88% triggers nothing.
        assert_eq!(notify_default(&disk_info(88.0)), None);
        // With a user-lowered low threshold of 85%, 88% triggers a notice.
        let result = determine_notification_with_thresholds(&disk_info(88.0), 85.0, 92.0, 96.0);
        assert!(result.is_some());
        let (title, _, _) = result.unwrap();
        assert_eq!(title, "Low disk space notice");
    }

    #[test]
    fn test_diskspace_info_equality() {
        assert_eq!(disk_info(50.0), disk_info(50.0));
    }

    #[test]
    fn test_default_paths() {
        assert_eq!(DEFAULT_PATHS, &["/", "/home"]);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_bytes_to_human_never_panics(bytes in 0u64..1_000_000_000_000) {
            let result = bytes_to_human(bytes);
            assert!(!result.is_empty());
        }

        #[test]
        fn test_bytes_to_human_always_has_unit(bytes in 1u64..1_000_000_000_000) {
            let result = bytes_to_human(bytes);
            let last_char = result.chars().last().unwrap();
            assert!(last_char.is_alphabetic());
        }

        #[test]
        fn test_format_diskspace_message_never_panics(
            total in 1u64..1_000_000_000_000,
            used_percent in 0.0f32..100.0
        ) {
            let used = ((total as f32 * used_percent) / 100.0) as u64;
            let available = total.saturating_sub(used);
            let info = DiskSpaceInfo {
                path: "/test".to_string(),
                total_bytes: total,
                used_bytes: used,
                available_bytes: available,
                percent_used: used_percent,
            };
            let result = format_diskspace_message(&info);
            assert!(!result.is_empty());
            assert!(result.contains('%'));
        }

        #[test]
        fn test_determine_notification_consistent(
            total in 1u64..1_000_000_000_000,
            percent in 0.0f32..100.0
        ) {
            let used = ((total as f32 * percent) / 100.0) as u64;
            let available = total.saturating_sub(used);
            let info = DiskSpaceInfo {
                path: "/test".to_string(),
                total_bytes: total,
                used_bytes: used,
                available_bytes: available,
                percent_used: percent,
            };
            let result = determine_notification_with_thresholds(&info, 90.0, 95.0, 97.5);
            if percent >= 97.5 {
                assert!(result.is_some());
                if let Some((_, _, notif_type)) = result {
                    assert_eq!(notif_type, NotificationType::Error);
                }
            } else if percent < 90.0 {
                assert!(result.is_none());
            }
        }
    }
}
