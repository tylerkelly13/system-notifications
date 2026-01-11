//! Disk space monitoring
//!
//! Monitors disk space usage and sends notifications when thresholds are crossed.

use crate::common::{notify, App, NotificationType};
use sysinfo::Disks;
use std::path::Path;

/// Disk usage threshold levels (percentage)
const THRESHOLD_LOW: f64 = 90.0;
const THRESHOLD_VERY_LOW: f64 = 95.0;
const THRESHOLD_CRITICAL: f64 = 97.5;

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
    pub percent_used: f64,
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
            let value = bytes as f64 / *unit as f64;
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
    let disk = disks.iter().find(|d| {
        path_obj.starts_with(d.mount_point())
    })?;

    let total_bytes = disk.total_space();
    let available_bytes = disk.available_space();
    let used_bytes = total_bytes.saturating_sub(available_bytes);

    let percent_used = if total_bytes > 0 {
        (used_bytes as f64 / total_bytes as f64) * 100.0
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

/// Determines which notification should be sent based on disk usage.
///
/// # Arguments
///
/// * `info` - The disk space information
///
/// # Returns
///
/// * `Some((title, message, notification_type))` if a notification should be sent
/// * `None` if disk space is acceptable
pub fn determine_notification(info: &DiskSpaceInfo) -> Option<(String, String, NotificationType)> {
    let percent = info.percent_used;
    let message = format_diskspace_message(info);

    if percent >= THRESHOLD_CRITICAL {
        Some((
            "Disk space critical".to_string(),
            message,
            NotificationType::Error,
        ))
    } else if percent >= THRESHOLD_VERY_LOW {
        Some((
            "Low disk space warning".to_string(),
            message,
            NotificationType::Info,
        ))
    } else if percent >= THRESHOLD_LOW {
        Some((
            "Low disk space notice".to_string(),
            message,
            NotificationType::Info,
        ))
    } else {
        None
    }
}

/// Checks disk space for specified paths and sends notifications if needed.
///
/// # Arguments
///
/// * `paths` - Slice of filesystem paths to monitor
///
/// This is the main entry point for the disk space monitoring service.
pub fn check_and_notify(paths: &[&str]) {
    for path in paths {
        if let Some(info) = get_disk_space_info(path) {
            if let Some((title, message, notification_type)) = determine_notification(&info) {
                notify(notification_type, App::DiskSpace, &title, &message);
            }
        }
    }
}

/// Checks disk space for default paths and sends notifications if needed.
///
/// Monitors "/" and "/home" by default.
pub fn check_and_notify_defaults() {
    check_and_notify(DEFAULT_PATHS);
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let info = DiskSpaceInfo {
            path: "/".to_string(),
            total_bytes: 100000000000,
            used_bytes: 90000000000,
            available_bytes: 10000000000,
            percent_used: 90.0,
        };

        let msg = format_diskspace_message(&info);
        assert!(msg.contains("/"));
        assert!(msg.contains("90.0%"));
        assert!(msg.contains("full"));
    }

    #[test]
    fn test_determine_notification_ok() {
        let info = DiskSpaceInfo {
            path: "/".to_string(),
            total_bytes: 100000000000,
            used_bytes: 50000000000,
            available_bytes: 50000000000,
            percent_used: 50.0,
        };

        assert_eq!(determine_notification(&info), None);
    }

    #[test]
    fn test_determine_notification_low() {
        let info = DiskSpaceInfo {
            path: "/".to_string(),
            total_bytes: 100000000000,
            used_bytes: 91000000000,
            available_bytes: 9000000000,
            percent_used: 91.0,
        };

        let result = determine_notification(&info);
        assert!(result.is_some());
        let (title, message, notif_type) = result.unwrap();
        assert_eq!(title, "Low disk space notice");
        assert!(message.contains("91.0%"));
        assert_eq!(notif_type, NotificationType::Info);
    }

    #[test]
    fn test_determine_notification_very_low() {
        let info = DiskSpaceInfo {
            path: "/home".to_string(),
            total_bytes: 100000000000,
            used_bytes: 96000000000,
            available_bytes: 4000000000,
            percent_used: 96.0,
        };

        let result = determine_notification(&info);
        assert!(result.is_some());
        let (title, message, notif_type) = result.unwrap();
        assert_eq!(title, "Low disk space warning");
        assert!(message.contains("96.0%"));
        assert_eq!(notif_type, NotificationType::Info);
    }

    #[test]
    fn test_determine_notification_critical() {
        let info = DiskSpaceInfo {
            path: "/".to_string(),
            total_bytes: 100000000000,
            used_bytes: 98000000000,
            available_bytes: 2000000000,
            percent_used: 98.0,
        };

        let result = determine_notification(&info);
        assert!(result.is_some());
        let (title, message, notif_type) = result.unwrap();
        assert_eq!(title, "Disk space critical");
        assert!(message.contains("98.0%"));
        assert_eq!(notif_type, NotificationType::Error);
    }

    #[test]
    fn test_determine_notification_at_thresholds() {
        // Test exact threshold values
        let info_low = DiskSpaceInfo {
            path: "/".to_string(),
            total_bytes: 100000000000,
            used_bytes: 90000000000,
            available_bytes: 10000000000,
            percent_used: 90.0,
        };
        assert!(determine_notification(&info_low).is_some());

        let info_very_low = DiskSpaceInfo {
            path: "/".to_string(),
            total_bytes: 100000000000,
            used_bytes: 95000000000,
            available_bytes: 5000000000,
            percent_used: 95.0,
        };
        assert!(determine_notification(&info_very_low).is_some());

        let info_critical = DiskSpaceInfo {
            path: "/".to_string(),
            total_bytes: 100000000000,
            used_bytes: 97500000000,
            available_bytes: 2500000000,
            percent_used: 97.5,
        };
        assert!(determine_notification(&info_critical).is_some());
    }

    #[test]
    fn test_diskspace_info_equality() {
        let info1 = DiskSpaceInfo {
            path: "/".to_string(),
            total_bytes: 100000000000,
            used_bytes: 50000000000,
            available_bytes: 50000000000,
            percent_used: 50.0,
        };
        let info2 = DiskSpaceInfo {
            path: "/".to_string(),
            total_bytes: 100000000000,
            used_bytes: 50000000000,
            available_bytes: 50000000000,
            percent_used: 50.0,
        };
        assert_eq!(info1, info2);
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
            // Should end with a unit letter or B
            let last_char = result.chars().last().unwrap();
            assert!(last_char.is_alphabetic());
        }

        #[test]
        fn test_format_diskspace_message_never_panics(
            total in 1u64..1_000_000_000_000,
            used_percent in 0.0f64..100.0
        ) {
            let used = ((total as f64 * used_percent) / 100.0) as u64;
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
            percent in 0.0f64..100.0
        ) {
            let used = ((total as f64 * percent) / 100.0) as u64;
            let available = total.saturating_sub(used);

            let info = DiskSpaceInfo {
                path: "/test".to_string(),
                total_bytes: total,
                used_bytes: used,
                available_bytes: available,
                percent_used: percent,
            };

            let result = determine_notification(&info);

            // Verify notification logic
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
