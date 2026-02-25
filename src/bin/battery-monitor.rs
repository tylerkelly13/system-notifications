//! Battery monitor service binary
//!
//! Checks battery level and sends notifications when thresholds are crossed.

use system_notifier::common::{notify_error, App};
use system_notifier::config::Config;

fn main() {
    let battery_config = match Config::load() {
        Ok(cfg) => cfg.battery.unwrap_or_default(),
        Err(e) => {
            eprintln!("system-monitor: config error: {e}");
            notify_error(
                App::Battery,
                "System Monitor config error",
                "Your system-monitor config file is malformed. \
                 Check the file and run: journalctl --user -u battery-monitor",
            );
            return;
        }
    };
    system_notifier::battery::check_and_notify(&battery_config);
}
