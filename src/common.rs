//! Common notification utilities
//!
//! Provides shared types and functions for sending desktop notifications
//! using the freedesktop notification system through notify-rust.

use notify_rust::{Hint, Notification, Timeout, Urgency};

/// The type of notification to display.
///
/// Determines the urgency level and icon used for the notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    /// Informational notification (normal urgency)
    Info,
    /// Error notification (critical urgency)
    Error,
}

/// The application context for the notification.
///
/// Used to set the application name and customize notification appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum App {
    /// Battery level monitor
    Battery,
    /// Disk space monitor
    DiskSpace,
}

/// Returns the icon name for a notification type.
///
/// # Arguments
///
/// * `notification_type` - The type of notification
///
/// # Returns
///
/// * `"info"` for Info notifications
/// * `"error"` for Error notifications
pub fn get_icon(notification_type: &NotificationType) -> &'static str {
    match notification_type {
        NotificationType::Info => "info",
        NotificationType::Error => "error",
    }
}

/// Returns the urgency level for a notification type.
///
/// # Arguments
///
/// * `notification_type` - The type of notification
///
/// # Returns
///
/// * `Urgency::Normal` for Info notifications
/// * `Urgency::Critical` for Error notifications
pub fn get_urgency(notification_type: &NotificationType) -> Urgency {
    match notification_type {
        NotificationType::Info => Urgency::Normal,
        NotificationType::Error => Urgency::Critical,
    }
}

/// Returns the application name for a given app context.
///
/// # Arguments
///
/// * `app` - The application context
///
/// # Returns
///
/// * `"Battery Monitor"` for Battery
/// * `"Disk Space Monitor"` for DiskSpace
pub fn get_appname(app: &App) -> &'static str {
    match app {
        App::Battery => "Battery Monitor",
        App::DiskSpace => "Disk Space Monitor",
    }
}

/// Sends a desktop notification with the specified parameters.
///
/// # Arguments
///
/// * `notification_type` - The type of notification (Info or Error)
/// * `app` - The application context (Battery or DiskSpace)
/// * `title` - The notification title/summary
/// * `body` - The notification body text
///
/// # Notification Behavior
///
/// - Info notifications use normal urgency and the "info" icon
/// - Error notifications use critical urgency and the "error" icon
/// - All notifications remain on screen until dismissed
/// - Notifications are categorized as "system" notifications
///
/// # Example
///
/// ```no_run
/// # use system_notifier::common::{notify, NotificationType, App};
/// notify(
///     NotificationType::Info,
///     App::Battery,
///     "Battery Low",
///     "Battery level is below 15%"
/// );
/// ```
pub fn notify(notification_type: NotificationType, app: App, title: &str, body: &str) {
    let icon = get_icon(&notification_type);
    let urgency = get_urgency(&notification_type);
    let appname = get_appname(&app);

    let _ = Notification::new()
        .summary(title)
        .appname(appname)
        .body(body)
        .icon(icon)
        .timeout(Timeout::Never)
        .urgency(urgency)
        .hint(Hint::Category("system".to_string()))
        .show();
}

/// Sends an error notification with critical urgency.
///
/// Convenience wrapper around [`notify`] that always uses
/// [`NotificationType::Error`].
///
/// # Arguments
///
/// * `app` - The application context (Battery or DiskSpace)
/// * `title` - The error notification title
/// * `message` - The error message body
///
/// # Example
///
/// ```no_run
/// # use system_notifier::common::{notify_error, App};
/// notify_error(
///     App::Battery,
///     "Monitor Failed",
///     "Failed to read battery information"
/// );
/// ```
pub fn notify_error(app: App, title: &str, message: &str) {
    notify(NotificationType::Error, app, title, message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_icon_info() {
        assert_eq!(get_icon(&NotificationType::Info), "info");
    }

    #[test]
    fn test_get_icon_error() {
        assert_eq!(get_icon(&NotificationType::Error), "error");
    }

    #[test]
    fn test_get_urgency_info() {
        assert_eq!(get_urgency(&NotificationType::Info), Urgency::Normal);
    }

    #[test]
    fn test_get_urgency_error() {
        assert_eq!(get_urgency(&NotificationType::Error), Urgency::Critical);
    }

    #[test]
    fn test_get_appname_battery() {
        assert_eq!(get_appname(&App::Battery), "Battery Monitor");
    }

    #[test]
    fn test_get_appname_diskspace() {
        assert_eq!(get_appname(&App::DiskSpace), "Disk Space Monitor");
    }

    #[test]
    fn test_icon_and_urgency_consistency() {
        // Info notifications should have normal urgency
        let info_icon = get_icon(&NotificationType::Info);
        let info_urgency = get_urgency(&NotificationType::Info);
        assert_eq!(info_icon, "info");
        assert_eq!(info_urgency, Urgency::Normal);

        // Error notifications should have critical urgency
        let error_icon = get_icon(&NotificationType::Error);
        let error_urgency = get_urgency(&NotificationType::Error);
        assert_eq!(error_icon, "error");
        assert_eq!(error_urgency, Urgency::Critical);
    }

    #[test]
    fn test_appname_uniqueness() {
        // Each app should have a distinct name
        let battery_name = get_appname(&App::Battery);
        let diskspace_name = get_appname(&App::DiskSpace);
        assert_ne!(battery_name, diskspace_name);
    }

    #[test]
    fn test_appname_non_empty() {
        assert!(!get_appname(&App::Battery).is_empty());
        assert!(!get_appname(&App::DiskSpace).is_empty());
    }

    #[test]
    fn test_icon_non_empty() {
        assert!(!get_icon(&NotificationType::Info).is_empty());
        assert!(!get_icon(&NotificationType::Error).is_empty());
    }

    #[test]
    fn test_notification_type_equality() {
        assert_eq!(NotificationType::Info, NotificationType::Info);
        assert_eq!(NotificationType::Error, NotificationType::Error);
        assert_ne!(NotificationType::Info, NotificationType::Error);
    }

    #[test]
    fn test_app_equality() {
        assert_eq!(App::Battery, App::Battery);
        assert_eq!(App::DiskSpace, App::DiskSpace);
        assert_ne!(App::Battery, App::DiskSpace);
    }
}
