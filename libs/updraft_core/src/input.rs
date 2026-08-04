use crate::connection::{ConnectionSpec, ConnectionState, ExternalDeviceId};
use crate::core::{AirspaceLoadError, Core};
use crate::effect::Effect;
use crate::fix::Fix;
use crate::settings::{Locale, UnitSettings};
use crate::time::Timestamp;
use std::sync::Arc;
use updraft_airspace::AirspaceDataset;

mod private {
    pub trait Sealed {}
}

pub trait Input: private::Sealed + Send + 'static {
    type Response: Send + 'static;

    #[doc(hidden)]
    fn apply_to(self, core: &mut Core, at: Timestamp) -> Update<Self::Response>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Update<R> {
    pub effects: Vec<Effect>,
    pub response: R,
}

/// Activates one immutable canonical airspace dataset.
#[derive(Clone, Debug, PartialEq)]
pub struct ActivateAirspaceDataset {
    pub dataset: Arc<AirspaceDataset>,
    pub source_name: Option<String>,
}

impl ActivateAirspaceDataset {
    pub fn new(dataset: Arc<AirspaceDataset>, source_name: Option<String>) -> Self {
        Self {
            dataset,
            source_name,
        }
    }
}

/// Removes the active canonical airspace dataset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClearAirspaceDataset;

/// Marks a stored airspace source as unavailable without exposing its technical error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetAirspaceUnavailable {
    pub source_name: Option<String>,
    pub error: AirspaceLoadError,
}

/// Requests a shared snapshot of the active canonical airspace dataset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GetAirspaceSnapshot;

impl SetAirspaceUnavailable {
    pub fn new(source_name: Option<String>, error: AirspaceLoadError) -> Self {
        Self { source_name, error }
    }
}

impl Update<()> {
    pub fn empty() -> Self {
        Self::effects(Vec::new())
    }

    pub fn effects(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            response: (),
        }
    }

    pub fn with_response<R>(self, response: R) -> Update<R> {
        Update {
            effects: self.effects,
            response,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Start;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tick;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bytes {
    pub device_id: ExternalDeviceId,
    pub data: Vec<u8>,
}

impl Bytes {
    pub fn new(device_id: ExternalDeviceId, data: impl Into<Vec<u8>>) -> Self {
        Self {
            device_id,
            data: data.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectionChanged {
    pub device_id: ExternalDeviceId,
    pub state: ConnectionState,
}

impl ConnectionChanged {
    pub fn new(device_id: ExternalDeviceId, state: ConnectionState) -> Self {
        Self { device_id, state }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InternalGps {
    pub fix: Fix,
}

impl InternalGps {
    pub fn new(fix: Fix) -> Self {
        Self { fix }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetLocale {
    pub locale: Locale,
}

impl SetLocale {
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }
}

/// Replaces all application-wide display-unit selections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetUnits {
    pub units: UnitSettings,
}

impl SetUnits {
    pub fn new(units: UnitSettings) -> Self {
        Self { units }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddExternalDevice {
    pub spec: ConnectionSpec,
}

impl AddExternalDevice {
    pub fn new(spec: ConnectionSpec) -> Self {
        Self { spec }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeleteExternalDevice {
    pub device_id: ExternalDeviceId,
}

impl DeleteExternalDevice {
    pub fn new(device_id: ExternalDeviceId) -> Self {
        Self { device_id }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReorderExternalDevices {
    pub order: Vec<ExternalDeviceId>,
}

impl ReorderExternalDevices {
    pub fn new(order: impl Into<Vec<ExternalDeviceId>>) -> Self {
        Self {
            order: order.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditExternalDevice {
    pub device_id: ExternalDeviceId,
    pub spec: ConnectionSpec,
}

impl EditExternalDevice {
    pub fn new(device_id: ExternalDeviceId, spec: ConnectionSpec) -> Self {
        Self { device_id, spec }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetExternalDeviceEnabled {
    pub device_id: ExternalDeviceId,
    pub enabled: bool,
}

impl SetExternalDeviceEnabled {
    pub fn enabled(device_id: ExternalDeviceId) -> Self {
        Self {
            device_id,
            enabled: true,
        }
    }

    pub fn disabled(device_id: ExternalDeviceId) -> Self {
        Self {
            device_id,
            enabled: false,
        }
    }
}

impl private::Sealed for Start {}
impl private::Sealed for ActivateAirspaceDataset {}
impl private::Sealed for ClearAirspaceDataset {}
impl private::Sealed for SetAirspaceUnavailable {}
impl private::Sealed for GetAirspaceSnapshot {}
impl private::Sealed for Tick {}
impl private::Sealed for Bytes {}
impl private::Sealed for ConnectionChanged {}
impl private::Sealed for InternalGps {}
impl private::Sealed for SetLocale {}
impl private::Sealed for SetUnits {}
impl private::Sealed for AddExternalDevice {}
impl private::Sealed for DeleteExternalDevice {}
impl private::Sealed for ReorderExternalDevices {}
impl private::Sealed for EditExternalDevice {}
impl private::Sealed for SetExternalDeviceEnabled {}
