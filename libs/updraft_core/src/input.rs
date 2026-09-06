use crate::Core;
use crate::connection::{ConnectionSpec, ConnectionState, ExternalDeviceId};
use crate::effect::Effect;
use crate::fix::Fix;
use crate::settings::{Locale, UnitSettings};
use crate::time::Timestamp;
use std::sync::Arc;

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

/// Requests the catalog and its generation without copying geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GetAirspaceSnapshot;

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

/// Selects the persisted glide polar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetPolar {
    pub polar: crate::PolarId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetArrivalReserve {
    pub reserve: crate::ArrivalReserve,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetMacCready {
    pub mac_cready: crate::MacCready,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetBugs {
    pub bugs: crate::Bugs,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetBallast {
    pub ballast: crate::Ballast,
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
impl private::Sealed for GetAirspaceSnapshot {}
impl private::Sealed for Tick {}
impl private::Sealed for Bytes {}
impl private::Sealed for ConnectionChanged {}
impl private::Sealed for InternalGps {}
impl private::Sealed for SetLocale {}
impl private::Sealed for SetUnits {}
impl private::Sealed for SetPolar {}
impl private::Sealed for SetArrivalReserve {}
impl private::Sealed for SetMacCready {}
impl private::Sealed for SetBugs {}
impl private::Sealed for SetBallast {}
impl private::Sealed for AddExternalDevice {}
impl private::Sealed for DeleteExternalDevice {}
impl private::Sealed for ReorderExternalDevices {}
impl private::Sealed for EditExternalDevice {}
impl private::Sealed for SetExternalDeviceEnabled {}

/// Replaces the immutable collection after a durable source mutation.
#[derive(Clone, Debug)]
pub struct ReplaceWaypointCatalog(pub Arc<crate::WaypointCatalog>);

/// Returns a shared waypoint collection without cloning waypoint records.
#[derive(Clone, Copy, Debug)]
pub struct GetWaypointCatalog;

impl private::Sealed for ReplaceWaypointCatalog {}
impl private::Sealed for GetWaypointCatalog {}

#[derive(Clone, Copy, Debug)]
pub struct GetWaypointSnapshot;
impl private::Sealed for GetWaypointSnapshot {}

#[derive(Clone, Copy, Debug)]
pub struct GetGlideSnapshot;
impl private::Sealed for GetGlideSnapshot {}

/// Replaces the source catalog after a durable file change.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplaceAirspaceCatalog(pub Arc<crate::AirspaceCatalog>);
impl private::Sealed for ReplaceAirspaceCatalog {}
