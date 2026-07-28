use crate::connection::{ConnectionId, ConnectionSpec};
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
        connection: ConnectionId,
        spec: ConnectionSpec,
    },
    /// Tear a link down and stop reconnecting it.
    CloseConnection {
        connection: ConnectionId,
    },
    PersistSettings(Settings),
}

impl Effect {
    pub fn emit(topic: Topic) -> Self {
        Self::Emit(topic)
    }

    pub fn open(connection: ConnectionId, spec: ConnectionSpec) -> Self {
        Self::OpenConnection { connection, spec }
    }

    pub fn close(connection: ConnectionId) -> Self {
        Self::CloseConnection { connection }
    }

    pub fn persist_settings(settings: Settings) -> Self {
        Self::PersistSettings(settings)
    }
}
