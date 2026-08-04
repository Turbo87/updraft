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
//! [`AirStateEstimator`] is the entry point. It is incremental: every
//! [`Sample`] updates the internal filters and returns the current
//! [`AirState`], so the same code serves a live sensor stream and a
//! replayed recording.
//!
//! The estimate needs a [`GlidePolar`](updraft_polar::GlidePolar) for the
//! netto, because the netto is the vertical speed with the glider's own
//! sink rate added back.

mod estimator;
mod wind;

pub use estimator::{AirState, AirStateEstimator, Sample};
pub use wind::Wind;
