use updraft_units::Speed;

mod ema;
mod window;

pub use ema::ClimbEma;
pub use window::ClimbWindow;

/// Altitude-derived climb estimates in metres per second.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ClimbEstimates {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub average_20s: Speed,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub average_30s: Speed,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub normalized_ema: Speed,
}
