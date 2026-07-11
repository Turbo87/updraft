//! Parser for the line-based NMEA text protocol spoken by connected
//! flight instruments (GPS sources, varios, FLARM, ...).
//!
//! [`parse()`] is fed raw bytes from a connection and pulls one framed
//! sentence off the front at a time, handling framing, checksum
//! validation, and resynchronization internally. It is a pure function
//! over a caller-owned byte buffer: it never owns a connection, allocates
//! a stream, or knows which transport carried the bytes.
//!
//! Every sentence family the crate understands is decoded unconditionally
//! into a wire-faithful [`Message`]. A well-formed sentence of an
//! unrecognized type becomes [`Message::Unknown`] rather than being
//! dropped.

mod datetime;
mod field;
mod framing;
mod message;
mod sentences;

pub use datetime::{Date, Time};
pub use framing::{RejectReason, Step, parse};
pub use message::{Message, Talker, Unknown};
pub use sentences::{
    FlarmAircraftType, FlarmAlarmLevel, FlarmId, FlarmIdType, FlarmSource, Gga, GgaFixQuality, Gsa,
    GsaFixType, GsaSelectionMode, Lxwp0, Lxwp1, Lxwp2, Lxwp3, Lxwp3SpeedCommandMode,
    Lxwp3SwitchMode, Pflaa, Pflac, PflacQueryType, Pflau, PflauAlarmType, PflauGpsStatus, Pgrmz,
    PgrmzFixDimension, Plxv0, Plxv0Direction, Plxvc, PlxvcMessageType, Plxvf, Plxvs, PlxvsMode,
    Plxvtarg, PositioningMode, Rmc, RmcStatus,
};
