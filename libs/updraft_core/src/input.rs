use crate::connection::{ConnectionSpec, ConnectionState, ExternalDeviceId};
use crate::core::Core;
use crate::effect::Effect;
use crate::fix::Fix;
use crate::settings::Locale;
use crate::time::Timestamp;

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
impl private::Sealed for Tick {}
impl private::Sealed for Bytes {}
impl private::Sealed for ConnectionChanged {}
impl private::Sealed for InternalGps {}
impl private::Sealed for SetLocale {}
impl private::Sealed for AddExternalDevice {}
impl private::Sealed for DeleteExternalDevice {}
impl private::Sealed for ReorderExternalDevices {}
impl private::Sealed for EditExternalDevice {}
impl private::Sealed for SetExternalDeviceEnabled {}
