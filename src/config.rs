use serde::{Deserialize, Serialize};
use toml::from_str as from_toml_str;

/// Fills fields of `self` that are unset with the values from `other`, keeping values already set.
trait Merge {
    fn merge(&mut self, other: Self);
}

// Leaf values: `self` keeps its value, so a set `Option` is never overwritten.
macro_rules! impl_merge_leaf {
    ($($t:ty),*) => { $(impl Merge for $t { fn merge(&mut self, _other: Self) {} })* };
}

impl_merge_leaf!(f32, bool, Vec<String>);

// A `None` takes the other side's value; two `Some` values merge recursively, so a
// partial user section keeps its own fields and gets the rest from defaults.
impl<T: Merge> Merge for Option<T> {
    fn merge(&mut self, other: Self) {
        match (self.as_mut(), other) {
            (Some(left), Some(right)) => left.merge(right),
            (None, right) => *self = right,
            (Some(_), None) => {}
        }
    }
}

// Implements `Merge` for a struct by merging each listed field. List every field,
// or defaults won't fill the missing one.
macro_rules! impl_merge {
    ($t:ty { $($field:ident),* }) => {
        impl Merge for $t {
            fn merge(&mut self, other: Self) {
                $(self.$field.merge(other.$field);)*
            }
        }
    };
}

#[derive(Serialize, Deserialize, Default)]
pub struct DiskThreshold {
    pub low: Option<f32>,
    pub very_low: Option<f32>,
    pub critical: Option<f32>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct DiskConfig {
    pub paths: Option<Vec<String>>,
    pub thresholds: Option<DiskThreshold>,
    pub disabled: Option<bool>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct BatteryThreshold {
    pub low: Option<f32>,
    pub very_low: Option<f32>,
    pub critical: Option<f32>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct BatteryConfig {
    pub thresholds: Option<BatteryThreshold>,
    pub disabled: Option<bool>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub battery: Option<BatteryConfig>,
    pub diskspace: Option<DiskConfig>,
}

impl_merge!(DiskThreshold {
    low,
    very_low,
    critical
});
impl_merge!(DiskConfig {
    paths,
    thresholds,
    disabled
});
impl_merge!(BatteryThreshold {
    low,
    very_low,
    critical
});
impl_merge!(BatteryConfig {
    thresholds,
    disabled
});
impl_merge!(Config { battery, diskspace });

impl Config {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_str = std::fs::read_to_string(path)?;
        let config = from_toml_str(&config_str)?;
        Ok(config)
    }

    pub fn config_from_str(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = from_toml_str(s)?;
        Ok(config)
    }

    pub fn defaults() -> Self {
        Self {
            battery: Some(BatteryConfig {
                thresholds: Some(BatteryThreshold {
                    low: Some(15.0),
                    very_low: Some(10.0),
                    critical: Some(5.0),
                }),
                disabled: Some(false),
            }),
            diskspace: Some(DiskConfig {
                paths: Some(vec![String::from("/")]),
                thresholds: Some(DiskThreshold {
                    low: Some(90.0),
                    very_low: Some(95.0),
                    critical: Some(97.5),
                }),
                disabled: Some(false),
            }),
        }
    }

    // Deep-merge with defaults so every nested field is guaranteed to be Some.
    pub fn merge_with_defaults(mut self) -> Self {
        let default = Self::defaults();
        self.merge(default);
        self
    }

    /// Searches for a config file in the following order:
    ///   1. `~/.config/system-monitor.toml`
    ///   2. `~/.config/system-monitor/config.toml`
    ///   3. `~/.system-monitor.toml`
    ///
    /// Returns the first file found merged with defaults, or pure defaults if
    /// none are found. Returns `Err` if a file exists but cannot be parsed.
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let home = std::env::var("HOME")?;
        let candidates = [
            format!("{}/.config/system-monitor.toml", home),
            format!("{}/.config/system-monitor/config.toml", home),
            format!("{}/.system-monitor.toml", home),
        ];
        for path in &candidates {
            if std::path::Path::new(path).exists() {
                let loaded = Self::from_file(path)?;
                return Ok(loaded.merge_with_defaults());
            }
        }
        Ok(Self::defaults())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Default values ────────────────────────────────────────────────────────

    #[test]
    fn test_default_battery_values() {
        let cfg = Config::defaults();
        let battery = cfg.battery.unwrap();
        let t = battery.thresholds.unwrap();
        assert_eq!(t.low, Some(15.0));
        assert_eq!(t.very_low, Some(10.0));
        assert_eq!(t.critical, Some(5.0));
        assert_eq!(battery.disabled, Some(false));
    }

    #[test]
    fn test_default_diskspace_values() {
        let cfg = Config::defaults();
        let disk = cfg.diskspace.unwrap();
        let t = disk.thresholds.unwrap();
        assert_eq!(t.low, Some(90.0));
        assert_eq!(t.very_low, Some(95.0));
        assert_eq!(t.critical, Some(97.5));
        assert_eq!(disk.disabled, Some(false));
        assert_eq!(disk.paths, Some(vec!["/".to_string()]));
    }

    // ── merge_with_defaults ────────────────────────────────────────────────────

    #[test]
    fn test_merge_fills_missing_battery_section() {
        // No battery section at all — should be populated from defaults.
        let partial = Config {
            battery: None,
            diskspace: None,
        };
        let merged = partial.merge_with_defaults();
        let t = merged.battery.unwrap().thresholds.unwrap();
        assert_eq!(t.low, Some(15.0));
    }

    #[test]
    fn test_merge_preserves_disabled_and_fills_thresholds() {
        // User sets disabled but omits thresholds entirely.
        let partial = Config {
            battery: Some(BatteryConfig {
                disabled: Some(true),
                thresholds: None,
            }),
            diskspace: None,
        };
        let merged = partial.merge_with_defaults();
        let battery = merged.battery.unwrap();
        assert_eq!(battery.disabled, Some(true)); // user value kept
        let t = battery.thresholds.unwrap();
        assert_eq!(t.low, Some(15.0)); // filled from defaults
        assert_eq!(t.very_low, Some(10.0));
        assert_eq!(t.critical, Some(5.0));
    }

    #[test]
    fn test_merge_preserves_partial_threshold_override() {
        // User sets only `low`; the other two come from defaults.
        let partial = Config {
            battery: Some(BatteryConfig {
                disabled: None,
                thresholds: Some(BatteryThreshold {
                    low: Some(25.0),
                    very_low: None,
                    critical: None,
                }),
            }),
            diskspace: None,
        };
        let merged = partial.merge_with_defaults();
        let t = merged.battery.unwrap().thresholds.unwrap();
        assert_eq!(t.low, Some(25.0)); // user override
        assert_eq!(t.very_low, Some(10.0)); // from defaults
        assert_eq!(t.critical, Some(5.0)); // from defaults
    }

    #[test]
    fn merge_with_defaults_fills_every_field() {
        // Catches a field missing from an `impl_merge!` list.
        let merged = Config::default().merge_with_defaults();
        let battery = merged.battery.unwrap();
        let bt = battery.thresholds.unwrap();
        assert!(battery.disabled.is_some());
        assert!(bt.low.is_some() && bt.very_low.is_some() && bt.critical.is_some());
        let disk = merged.diskspace.unwrap();
        assert!(disk.paths.is_some() && disk.disabled.is_some());
        let dt = disk.thresholds.unwrap();
        assert!(dt.low.is_some() && dt.very_low.is_some() && dt.critical.is_some());
    }

    #[test]
    fn merge_fills_missing_nested_fields_in_both_sections() {
        // Partial user sections: each keeps its own fields and takes the rest from defaults.
        let partial = Config {
            battery: Some(BatteryConfig {
                thresholds: Some(BatteryThreshold {
                    low: None,
                    very_low: Some(11.0),
                    critical: None,
                }),
                disabled: None,
            }),
            diskspace: Some(DiskConfig {
                paths: None,
                thresholds: Some(DiskThreshold {
                    low: None,
                    very_low: None,
                    critical: Some(99.0),
                }),
                disabled: Some(true),
            }),
        };
        let merged = partial.merge_with_defaults();
        let battery = merged.battery.unwrap();
        let bt = battery.thresholds.unwrap();
        assert_eq!(
            (bt.low, bt.very_low, bt.critical),
            (Some(15.0), Some(11.0), Some(5.0))
        );
        assert_eq!(battery.disabled, Some(false));
        let disk = merged.diskspace.unwrap();
        assert_eq!(disk.paths, Some(vec!["/".to_string()]));
        assert_eq!(disk.disabled, Some(true));
        let dt = disk.thresholds.unwrap();
        assert_eq!(
            (dt.low, dt.very_low, dt.critical),
            (Some(90.0), Some(95.0), Some(99.0))
        );
    }

    // ── TOML parsing (inline strings) ─────────────────────────────────────────

    #[test]
    fn test_config_from_str_full_config() {
        let toml = r#"
            [battery]
            disabled = false
            [battery.thresholds]
            low = 20.0
            very_low = 12.0
            critical = 6.0
            [diskspace]
            paths = ["/", "/home"]
            [diskspace.thresholds]
            low = 85.0
            very_low = 92.0
            critical = 96.0
        "#;
        let cfg = Config::config_from_str(toml).unwrap().merge_with_defaults();
        let bt = cfg.battery.unwrap().thresholds.unwrap();
        assert_eq!(bt.low, Some(20.0));
        assert_eq!(bt.very_low, Some(12.0));
        assert_eq!(bt.critical, Some(6.0));
        let dt = cfg.diskspace.unwrap();
        assert_eq!(dt.paths, Some(vec!["/".to_string(), "/home".to_string()]));
        let dt = dt.thresholds.unwrap();
        assert_eq!(dt.low, Some(85.0));
        assert_eq!(dt.very_low, Some(92.0));
        assert_eq!(dt.critical, Some(96.0));
    }

    #[test]
    fn test_config_from_str_partial_merges_with_defaults() {
        let toml = r#"
            [battery.thresholds]
            low = 25.0
        "#;
        let cfg = Config::config_from_str(toml).unwrap().merge_with_defaults();
        let t = cfg.battery.unwrap().thresholds.unwrap();
        assert_eq!(t.low, Some(25.0)); // user value
        assert_eq!(t.very_low, Some(10.0)); // default
        assert_eq!(t.critical, Some(5.0)); // default
    }

    #[test]
    fn test_config_from_str_malformed_returns_error() {
        let result = Config::config_from_str("this is not : valid toml ][");
        assert!(result.is_err());
    }

    // ── TOML fixture files ────────────────────────────────────────────────────

    #[test]
    fn test_fixture_partial_battery_threshold() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/partial_battery_threshold.toml"
        );
        let cfg = Config::from_file(path).unwrap().merge_with_defaults();
        let t = cfg.battery.unwrap().thresholds.unwrap();
        assert_eq!(t.low, Some(25.0)); // from fixture
        assert_eq!(t.very_low, Some(10.0)); // default
        assert_eq!(t.critical, Some(5.0)); // default
    }

    #[test]
    fn test_fixture_battery_disabled() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/battery_disabled.toml"
        );
        let cfg = Config::from_file(path).unwrap().merge_with_defaults();
        let battery = cfg.battery.unwrap();
        assert_eq!(battery.disabled, Some(true));
        // Thresholds must still be populated even though disabled = true.
        let t = battery.thresholds.unwrap();
        assert_eq!(t.low, Some(15.0));
    }
}
