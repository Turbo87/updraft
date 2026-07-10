//! The application core: a single-owner state machine that performs no
//! I/O, spawns no threads, and reads no clocks. Hosts drive it one
//! [`Input`] at a time through [`App::handle`] and execute the returned
//! [`Effect`]s; see `docs/design/core.md`.

mod app;
mod flight;
mod protocol;
mod time;

pub use app::App;
pub use flight::{
    FlightChange, FlightInput, InvalidPosition, ObservationSource, OwnshipPosition,
    PositionObservation,
};
pub use protocol::{Change, Effect, Input, Snapshot, Update};
pub use time::MonotonicTime;
