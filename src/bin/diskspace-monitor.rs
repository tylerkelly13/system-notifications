//! Disk space monitor service binary
//!
//! Checks disk space usage and sends notifications when thresholds are crossed.

use system_notifier::diskspace::check_and_notify_defaults;

fn main() {
    check_and_notify_defaults();
}
