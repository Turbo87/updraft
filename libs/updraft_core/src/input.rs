use crate::connection::{ConnectionState, ExternalDeviceId};
use crate::fix::Fix;
use crate::settings::Locale;

/// Anything that may change core state.
///
/// Input kinds stay distinct rather than being flattened into one record
/// type, because they differ in trust, in semantics, and in what the
/// domain needs to know about their origin.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    /// The first input the shell sends. Produces the effects needed to
    /// bring configured connections up.
    Start,
    /// A periodic wake-up, for policy that depends on elapsed time rather
    /// than on new data. Nothing uses it yet.
    Tick,
    /// Raw bytes from a device link, tagged with which link produced them.
    Bytes {
        device_id: ExternalDeviceId,
        data: Vec<u8>,
    },
    /// The shell reporting what happened to a link it was asked to maintain.
    ConnectionChanged {
        device_id: ExternalDeviceId,
        state: ConnectionState,
    },
    /// A fix from the device's own GNSS receiver rather than a connected
    /// instrument. Which source a position came from is what later lets
    /// them be ranked against each other.
    InternalGps(Fix),
    SetLocale(Locale),
}

impl Input {
    pub fn bytes(device_id: ExternalDeviceId, data: impl Into<Vec<u8>>) -> Self {
        Self::Bytes {
            device_id,
            data: data.into(),
        }
    }

    pub fn connection_changed(device_id: ExternalDeviceId, state: ConnectionState) -> Self {
        Self::ConnectionChanged { device_id, state }
    }
}
