//! Air-mass state estimation for Updraft.
//!
//! A soaring pilot needs to know three things that no sensor measures
//! directly: how fast the glider gains energy ([total-energy vertical
//! speed](AirState::vertical_speed)), how fast the surrounding air rises
//! ([netto](AirState::netto)), and where the air moves horizontally
//! ([wind](Wind)). This crate derives all three from what a flight
//! recorder logs: GNSS track, ground speed and position accuracy,
//! barometric altitude, and true airspeed.
//!
//! [`AirStateEstimator`] is the entry point. Each input arrives on its
//! own call, at whatever rate its source produces it, and
//! [`state`](AirStateEstimator::state) reports the current [`AirState`].
//! The same code therefore serves a live sensor stream and a replayed
//! recording.
//!
//! The estimate needs a [`GlidePolar`](updraft_polar::GlidePolar) for the
//! netto, because the netto is the vertical speed with the glider's own
//! sink rate added back.

mod circling;
mod estimator;
mod height;
mod wind;

pub use estimator::{AirState, AirStateEstimator, Fix};
pub use wind::Wind;

/// Weight of a new value in an exponential filter with the given time
/// constant, for a sample interval that is not fixed.
fn smoothing_weight(interval: f64, time_constant: f64) -> f64 {
    1. - (-interval / time_constant).exp()
}
