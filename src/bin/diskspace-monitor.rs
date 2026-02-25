//! Disk space monitor service binary
//!
//! Checks disk space usage and sends notifications when thresholds are crossed.

use system_notifier::common::{notify_error, App};
use system_notifier::config::Config;

fn main() {
    let disk_config = match Config::load() {
        Ok(cfg) => cfg.diskspace.unwrap_or_default(),
        Err(e) => {
            eprintln!("system-monitor: config error: {e}");
            notify_error(
                App::DiskSpace,
                "System Monitor config error",
                "Your system-monitor config file is malformed. \
                 Check the file and run: journalctl --user -u diskspace-monitor",
            );
            return;
        }
    };
    system_notifier::diskspace::check_and_notify(&disk_config);
}
