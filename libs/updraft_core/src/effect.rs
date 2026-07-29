use crate::connection::{ConnectionSpec, ExternalDeviceId};
use crate::settings::Settings;
use crate::topic::Topic;

/// A request for work that crosses the process boundary.
///
/// Effects exist only for I/O. Pure derivation stays inline in
/// `Core::apply()`. The shell matches exhaustively, so
/// a new effect cannot be silently ignored.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Publish a topic to the frontend.
    Emit(Topic),
    /// Bring up and keep up a link. The shell owns reconnection and
    /// backoff until a matching [`Effect::CloseConnection`].
    OpenConnection {
        device_id: ExternalDeviceId,
        spec: ConnectionSpec,
    },
    /// Tear a link down and stop reconnecting it.
    CloseConnection {
        device_id: ExternalDeviceId,
    },
    PersistSettings(Settings),
}

impl Effect {
    pub fn emit(topic: Topic) -> Self {
        Self::Emit(topic)
    }

    pub fn open(device_id: ExternalDeviceId, spec: ConnectionSpec) -> Self {
        Self::OpenConnection { device_id, spec }
    }

    pub fn close(device_id: ExternalDeviceId) -> Self {
        Self::CloseConnection { device_id }
    }

    pub fn persist_settings(settings: Settings) -> Self {
        Self::PersistSettings(settings)
    }
}
