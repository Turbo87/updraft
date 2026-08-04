//! Air-mass state estimation for Updraft.
//!
//! A soaring pilot needs to know things that no sensor measures directly:
//! how fast the glider gains energy ([total-energy vertical
//! speed](AirState::vertical_speed)), how fast the surrounding air rises
//! ([netto](AirState::netto)), where the air moves horizontally
//! ([wind](Wind)), and how high the glider is above sea level
//! ([altitude](AirState::altitude)) without an altimeter setting to enter.
//! This crate derives them from a GNSS receiver, a barometer, and an
//! airspeed sensor, each of which is allowed to be absent or to arrive at
//! its own rate.
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
