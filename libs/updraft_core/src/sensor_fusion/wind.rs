use std::time::Duration;
use updraft_units::{Angle, Speed};

use super::sample::SampleAcceptance;

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

/// Variance of one airspeed measurement, in `(m/s)²`. The value covers
/// the airspeed error and the gusts and turbulence that the estimate
/// must not follow.
const MEASUREMENT_NOISE: f64 = 0.5;

/// Initial variance per component, in `(m/s)²`. It must be large enough
/// to not hold the filter back from the first usable measurements.
const INITIAL_VARIANCE: f64 = 100.;

/// The estimate counts as converged once the sum of both component
/// variances drops to this value, in `(m/s)²`.
const CONVERGED_VARIANCE: f64 = 1.;

/// Airspeed below which the measurement carries no wind information,
/// in m/s. It also keeps the estimate out of the taxi and launch phase.
const MIN_AIR_SPEED: Speed = Speed::from_meters_per_second(15.);

/// Largest innovation applied in one update, as a multiple of its standard
/// deviation.
const MAX_NORMALIZED_INNOVATION: f64 = 3.;

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
    east: Speed,
    north: Speed,
    /// Upper triangle of the symmetric covariance matrix.
    variance_east: f64,
    covariance: f64,
    variance_north: f64,
}

impl Default for WindFilter {
    fn default() -> Self {
        Self {
            east: Speed::ZERO,
            north: Speed::ZERO,
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
    pub fn predict(&mut self, interval: Duration) {
        self.variance_east += PROCESS_NOISE * interval.as_secs_f64();
        self.variance_north += PROCESS_NOISE * interval.as_secs_f64();
    }

    /// Folds one airspeed measurement into the estimate. The measurement
    /// is the scalar `‖ground velocity − wind‖`, so it constrains the
    /// wind along the current heading alone.
    pub fn update(
        &mut self,
        ground_east: Speed,
        ground_north: Speed,
        air_speed: Speed,
    ) -> SampleAcceptance {
        if air_speed < MIN_AIR_SPEED {
            return SampleAcceptance::Ignored;
        }

        let relative_east = ground_east - self.east;
        let relative_north = ground_north - self.north;
        let relative_speed = relative_east
            .as_meters_per_second()
            .hypot(relative_north.as_meters_per_second());
        if relative_speed < f64::EPSILON {
            return SampleAcceptance::Ignored;
        }

        // Derivative of the measured air speed by the wind components.
        let jacobian_east = -relative_east.as_meters_per_second() / relative_speed;
        let jacobian_north = -relative_north.as_meters_per_second() / relative_speed;

        let projected_east = self.variance_east * jacobian_east + self.covariance * jacobian_north;
        let projected_north =
            self.covariance * jacobian_east + self.variance_north * jacobian_north;
        let innovation_variance =
            jacobian_east * projected_east + jacobian_north * projected_north + MEASUREMENT_NOISE;

        let gain_east = projected_east / innovation_variance;
        let gain_north = projected_north / innovation_variance;
        let expected_air_speed = Speed::from_meters_per_second(relative_speed);
        let innovation = (air_speed - expected_air_speed).as_meters_per_second();
        let limit = MAX_NORMALIZED_INNOVATION * innovation_variance.sqrt();
        let bounded_innovation = Speed::from_meters_per_second(innovation.clamp(-limit, limit));

        self.east += gain_east * bounded_innovation;
        self.north += gain_north * bounded_innovation;
        if innovation.abs() > limit {
            return SampleAcceptance::Accepted;
        }
        self.variance_east -= gain_east * projected_east;
        self.covariance -= gain_east * projected_north;
        self.variance_north -= gain_north * projected_north;
        SampleAcceptance::Accepted
    }

    /// Folds a complete wind vector into the estimate, with an isotropic
    /// variance of `variance` on each component. A circle fit produces
    /// one of these where no usable airspeed measurement does.
    pub fn update_vector(&mut self, east: Speed, north: Speed, variance: f64) -> SampleAcceptance {
        if variance <= 0. {
            return SampleAcceptance::Ignored;
        }

        let (pe, c, pn) = (self.variance_east, self.covariance, self.variance_north);
        let determinant = (pe + variance) * (pn + variance) - c * c;
        if determinant <= 0. {
            return SampleAcceptance::Ignored;
        }

        // Gain of a two-dimensional update with H = I.
        let gain_ee = (pe * pn + pe * variance - c * c) / determinant;
        let gain_en = c * variance / determinant;
        let gain_nn = (pe * pn + pn * variance - c * c) / determinant;

        let innovation_east = east - self.east;
        let innovation_north = north - self.north;
        self.east += gain_ee * innovation_east + gain_en * innovation_north;
        self.north += gain_en * innovation_east + gain_nn * innovation_north;

        // P -= K·P, with K symmetric.
        self.variance_east -= gain_ee * pe + gain_en * c;
        self.covariance -= gain_ee * c + gain_en * pn;
        self.variance_north -= gain_en * c + gain_nn * pn;
        SampleAcceptance::Accepted
    }

    /// The wind vector in m/s towards east and north, or `None` while the
    /// estimate is still too uncertain to report.
    ///
    /// Before the filter converges, a consumer that drew its initial zero
    /// state as a calm day would be wrong rather than uncertain.
    pub fn vector(&self) -> Option<(Speed, Speed)> {
        let converged = self.variance_east + self.variance_north <= CONVERGED_VARIANCE;
        converged.then_some((self.east, self.north))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor_fusion::sample::SampleAcceptance::{Accepted, Ignored};
    use approx::assert_abs_diff_eq;
    use claims::{assert_le, assert_none, assert_some};
    use std::f64::consts::TAU;

    fn speed(value: f64) -> Speed {
        Speed::from_meters_per_second(value)
    }

    /// Flies one circle per `TURN_SECONDS` at `air_speed` through a wind
    /// of `(east, north)` m/s, and returns the estimate afterwards.
    fn circle(seconds: usize, east: f64, north: f64) -> WindFilter {
        const TURN_SECONDS: f64 = 20.;
        const AIR_SPEED: f64 = 30.;

        let mut filter = WindFilter::default();
        for second in 0..seconds {
            let heading = TAU * second as f64 / TURN_SECONDS;
            if second > 0 {
                filter.predict(Duration::from_secs(1));
            }
            assert_eq!(
                filter.update(
                    speed(east + AIR_SPEED * heading.sin()),
                    speed(north + AIR_SPEED * heading.cos()),
                    speed(AIR_SPEED),
                ),
                Accepted
            );
        }
        filter
    }

    #[test]
    fn one_circle_recovers_the_wind_vector() {
        let (east, north) = assert_some!(circle(20, -6., -8.).vector());

        assert_abs_diff_eq!(east, speed(-6.), epsilon = 0.5);
        assert_abs_diff_eq!(north, speed(-8.), epsilon = 0.5);
    }

    #[test]
    fn estimate_survives_glide_without_measurements() {
        let mut filter = circle(20, -6., -8.);

        // Half an hour of straight flight with no measurement at all.
        // The process noise widens the estimate but not past the gate,
        // because the slow heading changes of a glide can support it.
        for _ in 0..1800 {
            filter.predict(Duration::from_secs(1));
        }

        assert_some!(filter.vector());
    }

    #[test]
    fn unconverged_estimate_is_not_reported() {
        assert_none!(circle(3, -6., -8.).vector());
    }

    #[test]
    fn taxi_speeds_leave_the_estimate_untouched() {
        let mut filter = WindFilter::default();
        filter.predict(Duration::from_secs(1));
        let air_speed = Speed::from_kilometers_per_hour(20.);
        assert_eq!(filter.update(speed(5.), Speed::ZERO, air_speed), Ignored);

        assert_none!(filter.vector());
    }

    #[test]
    fn one_outlier_has_a_bounded_effect() {
        let mut filter = circle(20, -6., -8.);
        let (east_before, north_before) = assert_some!(filter.vector());

        assert_eq!(filter.update(speed(24.), speed(-8.), speed(300.)), Accepted);

        let (east_after, north_after) = assert_some!(filter.vector());
        let change = (east_after - east_before)
            .as_meters_per_second()
            .hypot((north_after - north_before).as_meters_per_second());
        assert_le!(change, 0.5);
    }
}
