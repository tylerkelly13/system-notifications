//! Battery level monitoring
//!
//! Monitors battery levels and sends notifications when thresholds are crossed.

use crate::common::{notify, App, NotificationType};
use crate::config::BatteryConfig;
use battery::{Manager, State};

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
    let percent = (battery
        .state_of_charge()
        .get::<battery::units::ratio::percent>())
    .min(100.0)
    .max(0.0);

    // Determine if plugged in based on state
    let plugged_in = matches!(battery.state(), State::Charging | State::Full);

    // Get time remaining (only meaningful when discharging)
    let time_remaining_secs = if !plugged_in {
        battery
            .time_to_empty()
            .map(|duration| duration.get::<battery::units::time::second>() as u64)
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
pub fn format_battery_message(
    percentage: f32,
    time_remaining: Option<u64>,
    is_critical: bool,
) -> String {
    let mut message = format!("Your battery is at {}%", percentage);

    if is_critical {
        message.push_str(", plug in your device or prepare for shutdown");
    }

    if let Some(secs) = time_remaining {
        message.push_str(&format!(
            ". You have approximately {} of power remaining",
            secs_to_hours(secs)
        ));
    }

    message.push('.');
    message
}

/// Determines which notification should be sent based on battery level and explicit thresholds.
fn determine_notification_with_thresholds(
    battery: &BatteryInfo,
    low: f32,
    very_low: f32,
    critical: f32,
) -> Option<(String, String, NotificationType)> {
    if battery.plugged_in {
        return None;
    }

    let percent = battery.percent;

    if percent <= critical {
        let message = format_battery_message(percent.round(), battery.time_remaining_secs, true);
        Some((
            "Low battery warning".to_string(),
            message,
            NotificationType::Error,
        ))
    } else if percent <= very_low {
        let message = format_battery_message(percent.round(), battery.time_remaining_secs, true);
        Some((
            "Low battery warning".to_string(),
            message,
            NotificationType::Info,
        ))
    } else if percent <= low {
        let message = format_battery_message(percent.round(), battery.time_remaining_secs, false);
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
/// If `config.disabled` is `true`, returns immediately without checking.
pub fn check_and_notify(config: &BatteryConfig) {
    if config.disabled.unwrap_or(false) {
        return;
    }

    // After Config::load() deep-merges with defaults, all threshold fields
    // are guaranteed to be Some. Panicking here would indicate a programming
    // error (check_and_notify called without a merged config).
    let t = config.thresholds.as_ref().unwrap();
    let low = t.low.unwrap();
    let very_low = t.very_low.unwrap();
    let critical = t.critical.unwrap();

    if let Some(battery) = get_battery_info() {
        if let Some((title, message, notification_type)) =
            determine_notification_with_thresholds(&battery, low, very_low, critical)
        {
            notify(notification_type, App::Battery, &title, &message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shorthand: call the internal helper with the configured default thresholds (15/10/5).
    fn notify_default(battery: &BatteryInfo) -> Option<(String, String, NotificationType)> {
        determine_notification_with_thresholds(battery, 15.0, 10.0, 5.0)
    }

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
        assert!(msg.contains("at 5%"));
        assert!(msg.contains("plug in"));
        assert!(msg.contains("0:10:00"));
    }

    #[test]
    fn test_format_battery_message_non_critical() {
        let msg = format_battery_message(15.0, Some(3600), false);
        assert!(msg.contains("at 15%"));
        assert!(!msg.contains("plug in"));
        assert!(msg.contains("1:00:00"));
    }

    #[test]
    fn test_format_battery_message_no_time() {
        let msg = format_battery_message(10.0, None, true);
        assert!(msg.contains("at 10%"));
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
        assert_eq!(notify_default(&battery), None);
    }

    #[test]
    fn test_determine_notification_critical() {
        let battery = BatteryInfo {
            plugged_in: false,
            percent: 4.0,
            time_remaining_secs: Some(300),
        };
        let (title, message, notif_type) = notify_default(&battery).unwrap();
        assert_eq!(title, "Low battery warning");
        assert!(message.contains("at 4%"));
        assert_eq!(notif_type, NotificationType::Error);
    }

    #[test]
    fn test_determine_notification_very_low() {
        let battery = BatteryInfo {
            plugged_in: false,
            percent: 8.0,
            time_remaining_secs: Some(600),
        };
        let (title, message, notif_type) = notify_default(&battery).unwrap();
        assert_eq!(title, "Low battery warning");
        assert!(message.contains("at 8%"));
        assert_eq!(notif_type, NotificationType::Info);
    }

    #[test]
    fn test_determine_notification_low() {
        let battery = BatteryInfo {
            plugged_in: false,
            percent: 12.0,
            time_remaining_secs: Some(1800),
        };
        let (title, message, notif_type) = notify_default(&battery).unwrap();
        assert_eq!(title, "Low battery notice");
        assert!(message.contains("at 12%"));
        assert_eq!(notif_type, NotificationType::Info);
    }

    #[test]
    fn test_determine_notification_ok() {
        let battery = BatteryInfo {
            plugged_in: false,
            percent: 50.0,
            time_remaining_secs: Some(7200),
        };
        assert_eq!(notify_default(&battery), None);
    }

    #[test]
    fn test_determine_notification_at_threshold() {
        let battery_critical = BatteryInfo {
            plugged_in: false,
            percent: 5.0,
            time_remaining_secs: Some(300),
        };
        assert!(notify_default(&battery_critical).is_some());

        let battery_very_low = BatteryInfo {
            plugged_in: false,
            percent: 10.0,
            time_remaining_secs: Some(600),
        };
        assert!(notify_default(&battery_very_low).is_some());

        let battery_low = BatteryInfo {
            plugged_in: false,
            percent: 15.0,
            time_remaining_secs: Some(1200),
        };
        assert!(notify_default(&battery_low).is_some());
    }

    #[test]
    fn test_determine_notification_custom_thresholds() {
        // Verify that custom thresholds from user config are respected.
        let battery = BatteryInfo {
            plugged_in: false,
            percent: 18.0,
            time_remaining_secs: None,
        };
        // With defaults (15/10/5), 18% triggers nothing.
        assert_eq!(notify_default(&battery), None);
        // With a user-raised low threshold of 20%, 18% triggers a notice.
        let result = determine_notification_with_thresholds(&battery, 20.0, 10.0, 5.0);
        assert!(result.is_some());
        let (title, _, _) = result.unwrap();
        assert_eq!(title, "Low battery notice");
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
            let battery = BatteryInfo { plugged_in: false, percent, time_remaining_secs: time_secs };
            let result = determine_notification_with_thresholds(&battery, 15.0, 10.0, 5.0);
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
            let battery = BatteryInfo { plugged_in: true, percent, time_remaining_secs: time_secs };
            assert_eq!(
                determine_notification_with_thresholds(&battery, 15.0, 10.0, 5.0),
                None
            );
        }
    }
}
