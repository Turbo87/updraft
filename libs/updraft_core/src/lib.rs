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
mod settings;
mod time;
mod topic;

pub use connection::{ConnectionSpec, ConnectionState, ExternalDeviceId};
pub use core::Core;
pub use decoder::Decoder;
pub use effect::Effect;
pub use external_device::{ExternalDeviceConfig, PublishedExternalDevice};
pub use fix::Fix;
pub use input::Input;
pub use settings::{Locale, Settings, SettingsSnapshot};
pub use time::Timestamp;
pub use topic::{Instruments, LatLon, Topic};
