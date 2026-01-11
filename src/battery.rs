//! Battery level monitoring
//!
//! Monitors battery levels and sends notifications when thresholds are crossed.

use crate::common::{notify, App, NotificationType};
use battery::{Manager, State};

/// Battery threshold levels (percentage)
const THRESHOLD_LOW: f32 = 15.0;
const THRESHOLD_VERY_LOW: f32 = 10.0;
const THRESHOLD_CRITICAL: f32 = 5.0;

/// Battery information
#[derive(Debug, Clone, PartialEq)]
pub struct BatteryInfo {
    /// Whether the battery is plugged in
    pub plugged_in: bool,
    /// Battery percentage (0.0 to 100.0)
    pub percent: f32,
    /// Estimated time remaining in seconds (None if charging or unavailable)
    pub time_remaining_secs: Option<u64>,
}

/// Formats seconds into hours:minutes:seconds format.
///
/// # Arguments
///
/// * `secs` - The number of seconds to format
///
/// # Returns
///
/// A string in the format "H:MM:SS"
///
/// # Example
///
/// ```
/// # use system_notifier::battery::secs_to_hours;
/// assert_eq!(secs_to_hours(3661), "1:01:01");
/// assert_eq!(secs_to_hours(90), "0:01:30");
/// ```
pub fn secs_to_hours(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    format!("{}:{:02}:{:02}", hours, minutes, seconds)
}

/// Retrieves current battery information from the system.
///
/// # Returns
///
/// * `Some(BatteryInfo)` if battery information is available
/// * `None` if no battery is detected or information cannot be retrieved
pub fn get_battery_info() -> Option<BatteryInfo> {
    let manager = Manager::new().ok()?;
    let mut batteries = manager.batteries().ok()?;

    // Use the first battery (most systems have only one)
    let battery = batteries.next()?.ok()?;

    // Calculate percentage (state_of_charge returns a ratio from 0.0 to 1.0)
    let percent = (battery.state_of_charge().get::<battery::units::ratio::percent>())
        .min(100.0)
        .max(0.0);

    // Determine if plugged in based on state
    let plugged_in = matches!(
        battery.state(),
        State::Charging | State::Full
    );

    // Get time remaining (only meaningful when discharging)
    let time_remaining_secs = if !plugged_in {
        battery.time_to_empty().map(|duration| duration.get::<battery::units::time::second>() as u64)
    } else {
        None
    };

    Some(BatteryInfo {
        plugged_in,
        percent,
        time_remaining_secs,
    })
}

/// Formats the battery notification message.
///
/// # Arguments
///
/// * `threshold` - The threshold percentage that was crossed
/// * `time_remaining` - Optional time remaining in seconds
/// * `is_critical` - Whether this is a critical level notification
///
/// # Returns
///
/// A formatted message string describing the battery status
pub fn format_battery_message(threshold: f32, time_remaining: Option<u64>, is_critical: bool) -> String {
    let mut message = format!("Your battery is below {}%", threshold);

    if is_critical {
        message.push_str(", plug in your device or prepare for shutdown");
    }

    if let Some(secs) = time_remaining {
        message.push_str(&format!(". You have approximately {} of battery remaining", secs_to_hours(secs)));
    }

    message.push('.');
    message
}

/// Determines which notification should be sent based on battery level.
///
/// # Arguments
///
/// * `battery` - The current battery information
///
/// # Returns
///
/// * `Some((title, message, notification_type))` if a notification should be sent
/// * `None` if no notification is needed (battery level is acceptable or device is charging)
pub fn determine_notification(battery: &BatteryInfo) -> Option<(String, String, NotificationType)> {
    // Do not notify if device is plugged in
    if battery.plugged_in {
        return None;
    }

    let percent = battery.percent;

    if percent <= THRESHOLD_CRITICAL {
        let message = format_battery_message(
            THRESHOLD_CRITICAL,
            battery.time_remaining_secs,
            true,
        );
        Some((
            "Low battery warning".to_string(),
            message,
            NotificationType::Error,
        ))
    } else if percent <= THRESHOLD_VERY_LOW {
        let message = format_battery_message(
            THRESHOLD_VERY_LOW,
            battery.time_remaining_secs,
            true,
        );
        Some((
            "Low battery warning".to_string(),
            message,
            NotificationType::Info,
        ))
    } else if percent <= THRESHOLD_LOW {
        let message = format_battery_message(
            THRESHOLD_LOW,
            battery.time_remaining_secs,
            false,
        );
        Some((
            "Low battery notice".to_string(),
            message,
            NotificationType::Info,
        ))
    } else {
        None
    }
}

/// Checks battery level and sends appropriate notification if needed.
///
/// This is the main entry point for the battery monitoring service.
pub fn check_and_notify() {
    match get_battery_info() {
        Some(battery) => {
            if let Some((title, message, notification_type)) = determine_notification(&battery) {
                notify(notification_type, App::Battery, &title, &message);
            }
        }
        None => {
            // Only notify if we cannot get battery info (might be a desktop system)
            // In production, you might want to log this instead
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secs_to_hours_zero() {
        assert_eq!(secs_to_hours(0), "0:00:00");
    }

    #[test]
    fn test_secs_to_hours_one_hour() {
        assert_eq!(secs_to_hours(3600), "1:00:00");
    }

    #[test]
    fn test_secs_to_hours_complex() {
        assert_eq!(secs_to_hours(3661), "1:01:01");
        assert_eq!(secs_to_hours(7384), "2:03:04");
    }

    #[test]
    fn test_secs_to_hours_minutes_only() {
        assert_eq!(secs_to_hours(90), "0:01:30");
        assert_eq!(secs_to_hours(600), "0:10:00");
    }

    #[test]
    fn test_format_battery_message_critical() {
        let msg = format_battery_message(5.0, Some(600), true);
        assert!(msg.contains("below 5%"));
        assert!(msg.contains("plug in"));
        assert!(msg.contains("0:10:00"));
    }

    #[test]
    fn test_format_battery_message_non_critical() {
        let msg = format_battery_message(15.0, Some(3600), false);
        assert!(msg.contains("below 15%"));
        assert!(!msg.contains("plug in"));
        assert!(msg.contains("1:00:00"));
    }

    #[test]
    fn test_format_battery_message_no_time() {
        let msg = format_battery_message(10.0, None, true);
        assert!(msg.contains("below 10%"));
        assert!(msg.contains("plug in"));
        assert!(!msg.contains("approximately"));
    }

    #[test]
    fn test_determine_notification_plugged_in() {
        let battery = BatteryInfo {
            plugged_in: true,
            percent: 5.0,
            time_remaining_secs: None,
        };
        assert_eq!(determine_notification(&battery), None);
    }

    #[test]
    fn test_determine_notification_critical() {
        let battery = BatteryInfo {
            plugged_in: false,
            percent: 4.0,
            time_remaining_secs: Some(300),
        };
        let result = determine_notification(&battery);
        assert!(result.is_some());
        let (title, message, notif_type) = result.unwrap();
        assert_eq!(title, "Low battery warning");
        assert!(message.contains("below 5%"));
        assert_eq!(notif_type, NotificationType::Error);
    }

    #[test]
    fn test_determine_notification_very_low() {
        let battery = BatteryInfo {
            plugged_in: false,
            percent: 8.0,
            time_remaining_secs: Some(600),
        };
        let result = determine_notification(&battery);
        assert!(result.is_some());
        let (title, message, notif_type) = result.unwrap();
        assert_eq!(title, "Low battery warning");
        assert!(message.contains("below 10%"));
        assert_eq!(notif_type, NotificationType::Info);
    }

    #[test]
    fn test_determine_notification_low() {
        let battery = BatteryInfo {
            plugged_in: false,
            percent: 12.0,
            time_remaining_secs: Some(1800),
        };
        let result = determine_notification(&battery);
        assert!(result.is_some());
        let (title, message, notif_type) = result.unwrap();
        assert_eq!(title, "Low battery notice");
        assert!(message.contains("below 15%"));
        assert_eq!(notif_type, NotificationType::Info);
    }

    #[test]
    fn test_determine_notification_ok() {
        let battery = BatteryInfo {
            plugged_in: false,
            percent: 50.0,
            time_remaining_secs: Some(7200),
        };
        assert_eq!(determine_notification(&battery), None);
    }

    #[test]
    fn test_determine_notification_at_threshold() {
        // Test exact threshold values
        let battery_critical = BatteryInfo {
            plugged_in: false,
            percent: 5.0,
            time_remaining_secs: Some(300),
        };
        assert!(determine_notification(&battery_critical).is_some());

        let battery_very_low = BatteryInfo {
            plugged_in: false,
            percent: 10.0,
            time_remaining_secs: Some(600),
        };
        assert!(determine_notification(&battery_very_low).is_some());

        let battery_low = BatteryInfo {
            plugged_in: false,
            percent: 15.0,
            time_remaining_secs: Some(1200),
        };
        assert!(determine_notification(&battery_low).is_some());
    }

    #[test]
    fn test_battery_info_equality() {
        let battery1 = BatteryInfo {
            plugged_in: false,
            percent: 50.0,
            time_remaining_secs: Some(3600),
        };
        let battery2 = BatteryInfo {
            plugged_in: false,
            percent: 50.0,
            time_remaining_secs: Some(3600),
        };
        assert_eq!(battery1, battery2);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_secs_to_hours_never_panics(secs in 0u64..86400) {
            let result = secs_to_hours(secs);
            assert!(!result.is_empty());
            assert!(result.contains(':'));
        }

        #[test]
        fn test_format_battery_message_never_panics(
            threshold in 0.0f32..100.0,
            secs in proptest::option::of(0u64..86400),
            is_critical: bool
        ) {
            let result = format_battery_message(threshold, secs, is_critical);
            assert!(!result.is_empty());
            assert!(result.contains('%'));
        }

        #[test]
        fn test_determine_notification_consistent(
            percent in 0.0f32..100.0,
            time_secs in proptest::option::of(0u64..86400)
        ) {
            let battery = BatteryInfo {
                plugged_in: false,
                percent,
                time_remaining_secs: time_secs,
            };

            let result = determine_notification(&battery);

            // Verify notification logic
            if percent <= 5.0 {
                assert!(result.is_some());
                if let Some((_, _, notif_type)) = result {
                    assert_eq!(notif_type, NotificationType::Error);
                }
            } else if percent > 15.0 {
                assert!(result.is_none());
            }
        }

        #[test]
        fn test_plugged_in_never_notifies(
            percent in 0.0f32..100.0,
            time_secs in proptest::option::of(0u64..86400)
        ) {
            let battery = BatteryInfo {
                plugged_in: true,
                percent,
                time_remaining_secs: time_secs,
            };

            assert_eq!(determine_notification(&battery), None);
        }
    }
}
