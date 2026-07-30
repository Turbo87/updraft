use serde::{Deserialize, Serialize};

use crate::Topic;
use crate::external_device::ExternalDeviceConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    De,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub locale: Option<Locale>,
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
