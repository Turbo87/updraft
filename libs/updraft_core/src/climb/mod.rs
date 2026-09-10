mod ema;
mod window;

pub use ema::ClimbEma;
pub use window::ClimbWindow;

/// Altitude-derived climb estimates in metres per second.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ClimbEstimates {
    pub average_20s: f64,
    pub average_30s: f64,
    pub normalized_ema: f64,
}
