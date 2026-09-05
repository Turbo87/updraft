use serde::{Deserialize, Serialize};

use crate::external_device::ExternalDeviceConfig;
use crate::{PolarId, Topic};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    De,
}

/// The unit used to display altitude values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum AltitudeUnit {
    #[default]
    #[serde(rename = "m")]
    Meters,
    #[serde(rename = "ft")]
    Feet,
}

/// The unit used to display distance values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DistanceUnit {
    #[default]
    #[serde(rename = "km")]
    Kilometers,
    #[serde(rename = "mi")]
    Miles,
    #[serde(rename = "nm")]
    NauticalMiles,
}

/// The unit used to display horizontal speed values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum SpeedUnit {
    #[default]
    #[serde(rename = "km/h")]
    KilometersPerHour,
    #[serde(rename = "kt")]
    Knots,
    #[serde(rename = "mph")]
    MilesPerHour,
}

/// The unit used to display vertical speed values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum VerticalSpeedUnit {
    #[default]
    #[serde(rename = "m/s")]
    MetersPerSecond,
    #[serde(rename = "kt")]
    Knots,
    #[serde(rename = "ft/min")]
    FeetPerMinute,
}

/// The application-wide display-unit selections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct UnitSettings {
    pub altitude: AltitudeUnit,
    pub distance: DistanceUnit,
    pub speed: SpeedUnit,
    pub vertical_speed: VerticalSpeedUnit,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub locale: Option<Locale>,
    #[serde(default)]
    pub polar: PolarId,
    #[serde(default)]
    pub units: UnitSettings,
}

impl Settings {
    pub fn as_topic(&self) -> Topic {
        Topic::Settings(*self)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    #[serde(flatten)]
    pub settings: Settings,
    #[serde(default)]
    pub external_devices: Vec<ExternalDeviceConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_units_preserve_metric_display_units() {
        assert_eq!(
            UnitSettings::default(),
            UnitSettings {
                altitude: AltitudeUnit::Meters,
                distance: DistanceUnit::Kilometers,
                speed: SpeedUnit::KilometersPerHour,
                vertical_speed: VerticalSpeedUnit::MetersPerSecond,
            }
        );
    }
}
