use std::time::Duration;

/// Weight of a new value in an exponential filter with the given time
/// constant, for a sample interval that is not fixed.
pub fn smoothing_weight(interval: Duration, time_constant: Duration) -> f64 {
    1. - (-interval.as_secs_f64() / time_constant.as_secs_f64()).exp()
}
