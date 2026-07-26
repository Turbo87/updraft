/// Identifies one link to an external device.
///
/// The identity travels with every byte the link produces, because
/// position-source arbitration and failover need to know which device a
/// value came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnectionId(pub u32);

/// How the shell should reach a device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionSpec {
    /// A TCP client link. Used for flight simulators, WiFi-attached
    /// instruments, and any device exposing a TCP server, as well as for
    /// feeding recorded NMEA during development.
    Tcp { host: String, port: u16 },
}

impl ConnectionSpec {
    pub fn tcp(host: impl Into<String>, port: u16) -> Self {
        Self::Tcp {
            host: host.into(),
            port,
        }
    }
}

/// What the shell reports back about a link.
///
/// The shell owns reconnection and backoff between an open and a close
/// effect, so `Disconnected` describes the current situation rather than a
/// request for the core to do anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnected,
}
