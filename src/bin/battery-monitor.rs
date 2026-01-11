//! Battery monitor service binary
//!
//! Checks battery level and sends notifications when thresholds are crossed.

use system_notifier::battery::check_and_notify;

fn main() {
    check_and_notify();
}
