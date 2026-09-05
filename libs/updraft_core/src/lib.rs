//! The deterministic Updraft core.
//!
//! The core owns shared application state and the decisions based on it.
//! It performs no I/O, spawns no threads, and reads no clocks.

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
mod polar;
mod sensor_fusion;
mod settings;
mod signal_state;
mod time;
mod topic;
mod traffic;
mod waypoints;

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
    DeleteExternalDevice, EditExternalDevice, GetAirspaceSnapshot, GetWaypointCatalog,
    GetWaypointSnapshot, Input, InternalGps, ReorderExternalDevices, ReplaceWaypointCatalog,
    SetAirspaceUnavailable, SetExternalDeviceEnabled, SetLocale, SetPolar, SetUnits, Start, Tick,
    Update,
};
pub use polar::{PolarId, UnknownPolar};
pub use settings::{
    AltitudeUnit, DistanceUnit, Locale, Settings, SettingsSnapshot, SpeedUnit, UnitSettings,
    VerticalSpeedUnit,
};
pub use time::Timestamp;
pub use topic::{
    AltitudeInstrument, DerivedAltitudeInstruments, DerivedBankInstruments,
    DerivedHeadingInstruments, DerivedInstruments, FixTime as PublishedFixTime, GpsInstruments,
    Instruments, LatLon, SpeedInstrument, Topic,
};
pub use traffic::{
    PublishedTrafficTarget, TrafficAlarmLevel, TrafficChanges, TrafficDelta, TrafficState,
    TrafficTarget, TrafficTargetId, TrafficTargetIdType, TrafficType, TrafficUpdate,
    target_from_pflaa,
};
pub use waypoints::{
    WaypointCatalog, WaypointDiagnostic, WaypointLoadError, WaypointSnapshot, WaypointSourceStatus,
    WaypointStatus,
};
