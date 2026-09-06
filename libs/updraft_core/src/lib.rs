//! The deterministic Updraft core.
//!
//! The core owns shared application state and the decisions based on it.
//! It performs no I/O, spawns no threads, and reads no clocks.

mod airspace;
mod arrival_reserve;
#[cfg(feature = "ts")]
pub mod bindings;
mod connection;
mod connection_diagnostics;
mod core;
mod decoder;
mod effect;
mod external_device;
mod fix;
mod glide;
mod glide_performance;
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

pub use airspace::{
    AirspaceCatalog, AirspaceLoadError, AirspaceSnapshot, AirspaceSourceStatus, AirspaceState,
    AirspaceStatus,
};
pub use arrival_reserve::{ArrivalReserve, InvalidArrivalReserve};
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
pub use fix::{Fix, FixTime, UtcInstant, UtcTime};
pub use glide::{GlideSnapshot, WaypointArrival, WaypointArrivalEntry, WaypointArrivals};
pub use glide_performance::{
    Ballast, Bugs, GlidePerformance, InvalidBallast, InvalidBugs, InvalidMacCready, MacCready,
};
pub use input::{
    ActivateAirspaceDataset, AddExternalDevice, Bytes, ClearAirspaceDataset, ConnectionChanged,
    DeleteExternalDevice, EditExternalDevice, GetAirspaceSnapshot, GetGlideSnapshot,
    GetWaypointCatalog, GetWaypointSnapshot, Input, InternalGps, ReorderExternalDevices,
    ReplaceAirspaceCatalog, ReplaceWaypointCatalog, SetAirspaceUnavailable, SetArrivalReserve,
    SetBallast, SetBugs, SetExternalDeviceEnabled, SetLocale, SetMacCready, SetPolar, SetUnits,
    Start, Tick, Update,
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
