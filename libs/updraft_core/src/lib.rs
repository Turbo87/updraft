//! The deterministic Updraft core.
//!
//! The core owns shared application state and the decisions based on it.
//! It performs no I/O, spawns no threads, and reads no clocks.

mod air;
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

pub use connection::{
    ConnectionSpec, ConnectionState, ExternalDeviceId, STANDARD_SPP_SERVICE_UUID,
};
pub use core::{AirspaceLoadError, AirspaceState, AirspaceStatus, Core};
pub use decoder::Decoder;
pub use effect::Effect;
pub use external_device::{
    ExternalDeviceConfig, InvalidExternalDeviceOrder, PublishedExternalDevice,
    UnknownExternalDevice,
};
pub use fix::{Fix, FixTime, UtcInstant, UtcTime};
pub use input::{
    ActivateAirspaceDataset, AddExternalDevice, Bytes, ClearAirspaceDataset, ConnectionChanged,
    DeleteExternalDevice, EditExternalDevice, GetAirspaceSnapshot, Input, InternalGps,
    ReorderExternalDevices, SetAirspaceUnavailable, SetExternalDeviceEnabled, SetLocale, SetUnits,
    Start, Tick, Update,
};
pub use settings::{
    AltitudeUnit, DistanceUnit, Locale, Settings, SettingsSnapshot, SpeedUnit, UnitSettings,
    VerticalSpeedUnit,
};
pub use time::Timestamp;
pub use topic::{
    FixTime as PublishedFixTime, GpsInstruments, Instruments, LatLon, PressureAltitudeInstruments,
    Topic, TrueAirspeedInstruments,
};
pub use traffic::{
    PublishedTrafficTarget, TrafficAlarmLevel, TrafficChanges, TrafficDelta, TrafficState,
    TrafficTarget, TrafficTargetId, TrafficTargetIdType, TrafficType, TrafficUpdate,
    target_from_pflaa,
};
