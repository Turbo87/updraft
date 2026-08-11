use updraft_units::{Angle, Length, Speed};

/// The horizontal movement of the air mass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wind {
    /// The direction the wind comes *from*, clockwise from true north.
    pub direction: Angle,
    /// How fast the air mass moves.
    pub speed: Speed,
}

/// Growth of the wind variance per second, in `(m/s)²`. It sets how fast
/// the estimate follows a real change of the wind, and therefore how much
/// it follows the noise of a single measurement.
const PROCESS_NOISE: f64 = 2e-4;

/// Variance of one airspeed measurement, in `(m/s)²`, for a fix of
/// perfect position accuracy. The value covers the airspeed error and
/// the gusts and turbulence that the estimate must not follow. A fix at
/// [`REFERENCE_ACCURACY`] carries twice it.
const MEASUREMENT_NOISE: f64 = 0.5;

/// The GNSS position accuracy that doubles the measurement variance, in
/// metres. A worse accuracy scales it up quadratically.
const REFERENCE_ACCURACY: f64 = 15.;

/// Initial variance per component, in `(m/s)²`. It must be large enough
/// to not hold the filter back from the first usable measurements.
const INITIAL_VARIANCE: f64 = 100.;

/// The estimate counts as converged once the sum of both component
/// variances drops to this value, in `(m/s)²`.
const CONVERGED_VARIANCE: f64 = 1.;

/// Airspeed below which the measurement carries no wind information,
/// in m/s. It also keeps the estimate out of the taxi and launch phase.
const MIN_AIR_SPEED: f64 = 15.;

/// Estimates the wind vector from the difference between ground velocity
/// and true airspeed.
///
/// A glider without a compass cannot measure its heading, so a single
/// sample only says that the wind lies on a circle: the ground velocity
/// minus an air velocity of known length and unknown direction. This
/// filter treats each sample as one scalar measurement,
/// `TAS = ‖ground velocity − wind‖`, and tracks the two wind components
/// with an extended Kalman filter.
///
/// The measurement only constrains the wind along the current heading,
/// so a turn is what makes the wind observable. In a full circle the
/// heading sweeps every direction and the estimate converges within one
/// circle. In straight flight the along-heading component stays
/// corrected while the crosswind component only drifts with the process
/// noise, which is what the slow heading changes of a glide can support.
#[derive(Clone, Copy, Debug)]
pub struct WindFilter {
    east: f64,
    north: f64,
    /// Upper triangle of the symmetric covariance matrix.
    variance_east: f64,
    covariance: f64,
    variance_north: f64,
}

impl Default for WindFilter {
    fn default() -> Self {
        Self {
            east: 0.,
            north: 0.,
            variance_east: INITIAL_VARIANCE,
            covariance: 0.,
            variance_north: INITIAL_VARIANCE,
        }
    }
}

impl WindFilter {
    /// Lets the estimate age by `interval` seconds. It is called once per
    /// fix, whatever measurements that fix carries, so that the wind grows
    /// uncertain at the same rate whether it is being measured or not.
    pub fn predict(&mut self, interval: f64) {
        self.variance_east += PROCESS_NOISE * interval;
        self.variance_north += PROCESS_NOISE * interval;
    }

    /// Folds one airspeed measurement into the estimate. The measurement
    /// is the scalar `‖ground velocity − wind‖`, so it constrains the
    /// wind along the current heading alone.
    pub fn update(
        &mut self,
        ground_east: f64,
        ground_north: f64,
        air_speed: Speed,
        position_accuracy: Length,
    ) {
        // A value that is not a number would stay in the state for the
        // rest of the flight, and a comparison against it is false, so
        // the range check alone would let it through. An infinity has to
        // be rejected as well, not only a NaN.
        let air_speed = air_speed.as_meters_per_second();
        if !air_speed.is_finite() || air_speed < MIN_AIR_SPEED {
            return;
        }

        let relative_east = ground_east - self.east;
        let relative_north = ground_north - self.north;
        let relative_speed = relative_east.hypot(relative_north);
        if !relative_speed.is_finite() || relative_speed < f64::EPSILON {
            return;
        }

        // Derivative of the measured air speed by the wind components.
        let jacobian_east = -relative_east / relative_speed;
        let jacobian_north = -relative_north / relative_speed;

        let accuracy_factor = (position_accuracy.as_meters() / REFERENCE_ACCURACY).powi(2);
        let noise = MEASUREMENT_NOISE * (1. + accuracy_factor.max(0.));

        let projected_east = self.variance_east * jacobian_east + self.covariance * jacobian_north;
        let projected_north =
            self.covariance * jacobian_east + self.variance_north * jacobian_north;
        let innovation_variance =
            jacobian_east * projected_east + jacobian_north * projected_north + noise;

        let gain_east = projected_east / innovation_variance;
        let gain_north = projected_north / innovation_variance;
        let innovation = air_speed - relative_speed;

        self.east += gain_east * innovation;
        self.north += gain_north * innovation;
        self.variance_east -= gain_east * projected_east;
        self.covariance -= gain_east * projected_north;
        self.variance_north -= gain_north * projected_north;
    }

    /// The wind vector in m/s towards east and north, or `None` while the
    /// estimate is still too uncertain to report.
    ///
    /// Before the first circle the state is still the zero it started
    /// at, and a consumer that drew that as a calm day would be wrong
    /// rather than uncertain.
    pub fn vector(&self) -> Option<(f64, f64)> {
        let converged = self.variance_east + self.variance_north <= CONVERGED_VARIANCE;
        converged.then_some((self.east, self.north))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some};
    use std::f64::consts::TAU;

    /// Flies one circle per `TURN_SECONDS` at `air_speed` through a wind
    /// of `(east, north)` m/s, and returns the estimate afterwards.
    fn circle(seconds: usize, east: f64, north: f64) -> WindFilter {
        const TURN_SECONDS: f64 = 20.;
        const AIR_SPEED: f64 = 30.;

        let mut filter = WindFilter::default();
        for second in 0..seconds {
            let heading = TAU * second as f64 / TURN_SECONDS;
            if second > 0 {
                filter.predict(1.);
            }
            filter.update(
                east + AIR_SPEED * heading.sin(),
                north + AIR_SPEED * heading.cos(),
                Speed::from_meters_per_second(AIR_SPEED),
                Length::from_meters(REFERENCE_ACCURACY),
            );
        }
        filter
    }

    #[test]
    fn one_circle_recovers_the_wind_vector() {
        let (east, north) = assert_some!(circle(20, -6., -8.).vector());

        assert_abs_diff_eq!(east, -6., epsilon = 0.5);
        assert_abs_diff_eq!(north, -8., epsilon = 0.5);
    }

    #[test]
    fn an_estimate_survives_a_glide_with_nothing_to_measure() {
        let mut filter = circle(20, -6., -8.);

        // Half an hour of straight flight with no measurement at all.
        // The process noise widens the estimate but not past the gate,
        // because the slow heading changes of a glide can support it.
        for _ in 0..1800 {
            filter.predict(1.);
        }

        assert_some!(filter.vector());
    }

    #[test]
    fn an_unconverged_estimate_is_not_reported() {
        assert_none!(circle(3, -6., -8.).vector());
    }

    #[test]
    fn taxi_speeds_leave_the_estimate_untouched() {
        let mut filter = WindFilter::default();
        filter.predict(1.);
        filter.update(
            5.,
            0.,
            Speed::from_kilometers_per_hour(20.),
            Length::from_meters(REFERENCE_ACCURACY),
        );

        assert_none!(filter.vector());
    }
}
