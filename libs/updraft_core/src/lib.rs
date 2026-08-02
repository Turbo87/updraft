//! The deterministic Updraft core.
//!
//! The core owns shared application state and the decisions based on it.
//! It performs no I/O, spawns no threads, and reads no clocks.

mod airspace;
#[cfg(feature = "ts")]
pub mod bindings;
mod connection;
mod connection_diagnostics;
mod core;
mod decoder;
mod effect;
mod external_device;
mod fix;
mod input;
mod ownship;
mod settings;
mod time;
mod topic;
mod traffic;

pub use airspace::{
    Airspace, AirspaceAltitude, AirspaceClass, AirspaceDataset, AirspaceGeometryError, AirspaceId,
    AirspaceImportError, AirspaceParseError, AirspacePolygon, AirspaceType,
};
pub use connection::{
    ConnectionSpec, ConnectionState, ExternalDeviceId, STANDARD_SPP_SERVICE_UUID,
};
pub use core::Core;
pub use decoder::Decoder;
pub use effect::Effect;
pub use external_device::{
    ExternalDeviceConfig, InvalidExternalDeviceOrder, PublishedExternalDevice,
    UnknownExternalDevice,
};
pub use fix::Fix;
pub use input::{
    AddExternalDevice, Bytes, ConnectionChanged, DeleteExternalDevice, EditExternalDevice, Input,
    InternalGps, ReorderExternalDevices, SetExternalDeviceEnabled, SetLocale, SetUnits, Start,
    Tick, Update,
};
pub use ownship::OwnshipState;
pub use settings::{
    AltitudeUnit, DistanceUnit, Locale, Settings, SettingsSnapshot, SpeedUnit, UnitSettings,
    VerticalSpeedUnit,
};
pub use time::Timestamp;
pub use topic::{Instruments, LatLon, Topic};
pub use traffic::{
    PublishedTrafficTarget, TrafficAlarmLevel, TrafficChanges, TrafficDelta, TrafficState,
    TrafficTarget, TrafficTargetId, TrafficTargetIdType, TrafficType, TrafficUpdate,
    target_from_pflaa,
};
