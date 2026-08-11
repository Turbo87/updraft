//! Air-mass state estimation for Updraft.
//!
//! A soaring pilot needs to know things that no sensor measures directly,
//! starting with how fast the glider gains height. This crate derives
//! them from a GNSS receiver, a barometer, and an airspeed sensor, each
//! of which is allowed to be absent or to arrive at its own rate.
//!
//! [`AirStateEstimator`] is the entry point. Each input arrives on its
//! own call, at whatever rate its source produces it, and
//! [`state`](AirStateEstimator::state) reports the current [`AirState`].
//! The same code therefore serves a live sensor stream and a replayed
//! recording.

mod estimator;
mod height;
mod noise;
mod wind;

pub use estimator::{AirState, AirStateEstimator, Fix};
pub use wind::Wind;

/// Weight of a new value in an exponential filter with the given time
/// constant, for a sample interval that is not fixed.
fn smoothing_weight(interval: f64, time_constant: f64) -> f64 {
    1. - (-interval / time_constant).exp()
}
