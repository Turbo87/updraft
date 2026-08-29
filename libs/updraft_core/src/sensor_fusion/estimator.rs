use super::altitude::AltitudeFilter;
use super::sample::{AltitudeDomain, SampleAcceptance};
use super::vario::Vario;
use super::wind::{Wind, WindFilter};
use std::time::Duration;
use updraft_geo::LatLon;
use updraft_units::{Angle, EllipsoidAltitude, Length, MslAltitude, PressureAltitude, Speed};

const GRAVITY: f64 = 9.80665;
const MAX_WIND_AIR_SPEED_SKEW: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug)]
struct AirSpeedSample {
    time: Duration,
    speed: Speed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fix {
    pub track: Angle,
    pub ground_speed: Speed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FixAcceptance {
    Ignored,
    Predicted,
    RejectedWindMeasurement,
    AcceptedWithWindMeasurement,
}

/// Flight values derived from the available sensor inputs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Estimate {
    /// Rate of change of the selected altitude series. Positive means climbing.
    pub raw_vertical_speed: Option<Speed>,
    /// Smoothed vertical speed. Positive means climbing.
    pub vertical_speed: Option<Speed>,
    pub vario: Option<Speed>,
    pub wind: Option<Wind>,
    pub altitude: Option<MslAltitude>,
}

/// Derives flight values from timestamped physical measurements.
///
/// This layer owns numerical estimator state. [`SensorFusion`](super::SensorFusion)
/// owns selected-source continuity, freshness, and protocol projection.
#[derive(Clone, Debug)]
pub struct Estimator {
    altitude: AltitudeFilter,
    pressure_altitude_current: bool,
    vario: Vario,
    uncompensated: Vario,
    measured_air_speed: Option<AirSpeedSample>,
    gnss_time: Option<Duration>,
    position: Option<LatLon>,
    referenced_altitude: Option<EllipsoidAltitude>,
    wind: WindFilter,
    wind_air_speed_time: Option<Duration>,
    previous_fix_time: Option<Duration>,
}

impl Default for Estimator {
    fn default() -> Self {
        Self::new()
    }
}

impl Estimator {
    pub fn new() -> Self {
        Self {
            altitude: AltitudeFilter::default(),
            pressure_altitude_current: false,
            vario: Vario::default(),
            uncompensated: Vario::default(),
            measured_air_speed: None,
            gnss_time: None,
            position: None,
            referenced_altitude: None,
            wind: WindFilter::default(),
            wind_air_speed_time: None,
            previous_fix_time: None,
        }
    }

    /// Takes a barometric altitude against the 1013.25 hPa datum.
    pub fn pressure_altitude(
        &mut self,
        time: Duration,
        altitude: PressureAltitude,
    ) -> SampleAcceptance {
        let acceptance = self
            .uncompensated
            .acceptance(time, AltitudeDomain::Pressure);
        if acceptance == SampleAcceptance::Ignored {
            return acceptance;
        }

        let altitude = altitude.into_inner();
        self.pressure_altitude_current = true;
        let altitude = self.altitude.pressure(time, altitude);
        self.referenced_altitude = self
            .altitude
            .referenced_altitude()
            .map(EllipsoidAltitude::new);
        self.advance_vertical_speed(time, altitude, AltitudeDomain::Pressure)
    }

    pub fn clear_pressure_altitude(&mut self) {
        self.pressure_altitude_current = false;
    }

    /// Clears state that assumes continuity with one GNSS altitude source.
    pub fn reset_gnss_altitude(&mut self) {
        self.altitude.clear_gnss_reference();
        self.gnss_time = None;
        self.referenced_altitude = None;
    }

    /// Adds an ellipsoid-altitude sample when its timestamp advances.
    pub fn gnss_altitude(
        &mut self,
        time: Duration,
        altitude: EllipsoidAltitude,
    ) -> SampleAcceptance {
        if self.gnss_time.is_some_and(|previous| time <= previous) {
            return SampleAcceptance::Ignored;
        }
        self.gnss_time = Some(time);

        let altitude_value = altitude.into_inner();
        if self.pressure_altitude_current {
            self.altitude.gnss(time, altitude_value);
            self.referenced_altitude = self
                .altitude
                .referenced_altitude()
                .map(EllipsoidAltitude::new);
            SampleAcceptance::Accepted
        } else {
            let acceptance =
                self.advance_vertical_speed(time, altitude_value, AltitudeDomain::Gnss);
            if acceptance == SampleAcceptance::Accepted {
                self.referenced_altitude = Some(altitude);
            }
            acceptance
        }
    }

    /// Adds an airspeed sample when its timestamp advances.
    pub fn air_speed(&mut self, time: Duration, speed: Speed) -> SampleAcceptance {
        if self
            .measured_air_speed
            .is_some_and(|previous| time <= previous.time)
        {
            return SampleAcceptance::Ignored;
        }
        self.measured_air_speed = Some(AirSpeedSample { time, speed });
        SampleAcceptance::Accepted
    }

    fn altitude_msl(&self) -> Option<MslAltitude> {
        Some(updraft_egm96::ellipsoidal_to_msl(
            self.position?,
            self.referenced_altitude?,
        ))
    }

    /// Updates the position used to convert ellipsoid altitude to MSL altitude.
    pub fn position(&mut self, position: LatLon) {
        self.position = Some(position);
    }

    /// Adds a ground-velocity sample when its timestamp advances.
    pub fn fix(&mut self, time: Duration, fix: &Fix) -> FixAcceptance {
        if self
            .previous_fix_time
            .is_some_and(|previous| time <= previous)
        {
            return FixAcceptance::Ignored;
        }

        let (sin_track, cos_track) = fix.track.sin_cos();
        let east = fix.ground_speed * sin_track;
        let north = fix.ground_speed * cos_track;

        if let Some(previous) = self.previous_fix_time {
            self.wind.predict(time - previous);
        }
        let air_speed_sample = self.measured_air_speed.filter(|sample| {
            self.wind_air_speed_time != Some(sample.time)
                && sample.time.abs_diff(time) <= MAX_WIND_AIR_SPEED_SKEW
        });
        let wind_measurement = air_speed_sample.map_or(SampleAcceptance::Ignored, |sample| {
            let acceptance = self.wind.update(east, north, sample.speed);
            if acceptance == SampleAcceptance::Accepted {
                self.wind_air_speed_time = Some(sample.time);
            }
            acceptance
        });
        self.previous_fix_time = Some(time);
        if wind_measurement == SampleAcceptance::Accepted {
            FixAcceptance::AcceptedWithWindMeasurement
        } else if air_speed_sample.is_some() {
            FixAcceptance::RejectedWindMeasurement
        } else {
            FixAcceptance::Predicted
        }
    }

    pub fn clear_air_speed(&mut self) {
        self.reset_air_speed();
    }

    /// Clears state that assumes continuity with one airspeed source.
    pub fn reset_air_speed(&mut self) {
        self.measured_air_speed = None;
        self.vario = Vario::default();
    }

    fn advance_vertical_speed(
        &mut self,
        time: Duration,
        altitude: Length,
        domain: AltitudeDomain,
    ) -> SampleAcceptance {
        let acceptance = self.uncompensated.acceptance(time, domain);
        if acceptance == SampleAcceptance::Ignored {
            return acceptance;
        }
        let fusion = self.altitude.take_step();
        let uncompensated = self.uncompensated.advance(time, altitude, fusion, domain);
        debug_assert_eq!(uncompensated, acceptance);

        let Some(AirSpeedSample { speed, .. }) = self.measured_air_speed else {
            return acceptance;
        };
        let speed = speed.as_meters_per_second();
        let energy = Length::from_meters(speed * speed / (2. * GRAVITY));
        let compensated = self.vario.advance(time, altitude + energy, fusion, domain);
        debug_assert_eq!(compensated, acceptance);
        acceptance
    }

    /// Clears state that assumes continuity with one altitude source.
    pub fn reset_altitude(&mut self) {
        self.altitude = AltitudeFilter::default();
        self.pressure_altitude_current = false;
        self.vario = Vario::default();
        self.uncompensated = Vario::default();
        self.gnss_time = None;
        self.referenced_altitude = None;
    }

    pub fn reset_wind(&mut self) {
        self.wind = WindFilter::default();
        self.wind_air_speed_time = None;
        self.previous_fix_time = None;
    }

    pub fn estimate(&self) -> Estimate {
        let raw_vertical_speed = self.uncompensated.value();
        let vertical_speed = self.uncompensated.smoothed_value();
        let vario = self.measured_air_speed.and(self.vario.smoothed_value());
        let wind = self.measured_air_speed.and_then(|_| {
            self.wind.vector().map(|(east, north)| {
                let east = east.as_meters_per_second();
                let north = north.as_meters_per_second();
                Wind {
                    direction: Angle::from_radians((-east).atan2(-north)).normalized(),
                    speed: Speed::from_meters_per_second(east.hypot(north)),
                }
            })
        });
        Estimate {
            raw_vertical_speed,
            vertical_speed,
            vario,
            wind,
            altitude: self.altitude_msl(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FixAcceptance::{
        AcceptedWithWindMeasurement as FixWithWind, Ignored as FixIgnored,
        Predicted as FixPredicted,
    };
    use super::SampleAcceptance::{Accepted, Ignored};
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some};
    use updraft_units::Length;

    fn meters(value: f64) -> PressureAltitude {
        PressureAltitude::new(Length::from_meters(value))
    }

    fn add_air_speed(estimator: &mut Estimator, time: Duration, speed: Speed) {
        assert_eq!(estimator.air_speed(time, speed), Accepted);
    }

    fn fix(track: f64, ground_speed: f64) -> Fix {
        Fix {
            track: Angle::from_degrees(track),
            ground_speed: Speed::from_meters_per_second(ground_speed),
        }
    }

    fn climb(rate: f64, seconds: u64) -> Estimator {
        let mut estimator = Estimator::new();
        for second in 0..seconds {
            assert_eq!(
                estimator.pressure_altitude(
                    Duration::from_secs(second),
                    meters(1000. + rate * second as f64),
                ),
                Accepted
            );
        }
        estimator
    }

    #[test]
    fn first_altitude_produces_no_estimate() {
        let mut estimator = Estimator::new();
        assert_eq!(
            estimator.pressure_altitude(Duration::ZERO, meters(1000.)),
            Accepted
        );

        assert_none!(estimator.estimate().raw_vertical_speed);
    }

    #[test]
    fn steady_climb_produces_its_rate() {
        let estimate = climb(2., 60).estimate();
        let raw_vertical_speed = assert_some!(estimate.raw_vertical_speed);
        let vertical_speed = assert_some!(estimate.vertical_speed);

        assert_abs_diff_eq!(
            raw_vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
        assert_abs_diff_eq!(
            vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn altitude_gap_resets_only_above_the_maximum_interval() {
        let estimator = climb(2., 60);
        let mut at_boundary = estimator.clone();
        let mut above_boundary = estimator;

        assert_eq!(
            at_boundary.pressure_altitude(Duration::from_secs(89), meters(1178.)),
            Accepted
        );
        assert_eq!(
            above_boundary.pressure_altitude(
                Duration::from_secs(89) + Duration::from_nanos(1),
                meters(1178.),
            ),
            Accepted
        );

        assert_some!(at_boundary.estimate().raw_vertical_speed);
        assert_none!(above_boundary.estimate().raw_vertical_speed);
    }

    #[test]
    fn repeated_timestamp_keeps_the_first_altitude() {
        let mut estimator = Estimator::new();
        for second in 0..60 {
            let time = Duration::from_secs(second);
            let altitude = meters(1000. + 2. * second as f64);
            assert_eq!(estimator.pressure_altitude(time, altitude), Accepted);
            assert_eq!(estimator.pressure_altitude(time, meters(10_000.)), Ignored);
        }

        let raw_vertical_speed = assert_some!(estimator.estimate().raw_vertical_speed);
        assert_abs_diff_eq!(
            raw_vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn one_hertz_climb_has_the_fitted_vertical_speed_response() {
        let mut estimator = Estimator::new();
        assert_eq!(
            estimator.pressure_altitude(Duration::ZERO, meters(1000.)),
            Accepted
        );

        let expected = [
            0.309_636_243_492_350_97,
            0.685_243_993_565_065_4,
            1.026_970_418_232_237_4,
            1.303_327_156_625_063_5,
        ];
        for (second, expected) in (1..).zip(expected) {
            assert_eq!(
                estimator.pressure_altitude(
                    Duration::from_secs(second),
                    meters(1000. + 2. * second as f64),
                ),
                Accepted
            );
            assert_abs_diff_eq!(
                assert_some!(estimator.estimate().vertical_speed),
                Speed::from_meters_per_second(expected),
                epsilon = 1e-9
            );
        }
    }

    #[test]
    fn coarse_timestamps_preserve_the_vertical_speed() {
        let mut estimator = Estimator::new();
        for half in 0..120 {
            // A 2 Hz source whose timestamps resolve to whole seconds:
            // every second arrives twice, and the altitude moves between
            // the two.
            let acceptance = estimator
                .pressure_altitude(Duration::from_secs(half / 2), meters(1000. + half as f64));
            assert_eq!(acceptance, if half % 2 == 0 { Accepted } else { Ignored });
        }

        let raw_vertical_speed = assert_some!(estimator.estimate().raw_vertical_speed);
        assert_abs_diff_eq!(
            raw_vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.05
        );
    }

    #[test]
    fn older_timestamp_preserves_the_estimate_and_reference() {
        let mut estimator = climb(2., 30);
        let mut control = estimator.clone();
        let before = estimator.estimate();

        assert_eq!(
            estimator.pressure_altitude(Duration::from_secs(10), meters(10_000.)),
            Ignored
        );
        assert_eq!(estimator.estimate(), before);

        let time = Duration::from_secs(30);
        let altitude = meters(1060.);
        assert_eq!(estimator.pressure_altitude(time, altitude), Accepted);
        assert_eq!(control.pressure_altitude(time, altitude), Accepted);

        assert_eq!(estimator.estimate(), control.estimate());
    }

    #[test]
    fn sample_rates_converge_to_the_same_steady_climb() {
        let slow = climb(2., 11).estimate();
        let mut fast = Estimator::new();
        for tenth in 0..=100u64 {
            let time = Duration::from_millis(tenth * 100);
            assert_eq!(
                fast.pressure_altitude(time, meters(1000. + 0.2 * tenth as f64)),
                Accepted
            );
        }
        let fast = fast.estimate();

        let expected = Speed::from_meters_per_second(2.);
        assert_abs_diff_eq!(
            assert_some!(slow.raw_vertical_speed),
            expected,
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            assert_some!(fast.raw_vertical_speed),
            expected,
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            assert_some!(fast.vertical_speed),
            assert_some!(slow.vertical_speed),
            epsilon = 0.05
        );
    }

    fn pull_up(compensated: bool) -> Estimator {
        let mut estimator = Estimator::new();
        for second in 0..60 {
            let time = Duration::from_secs(second);
            let speed = (120. - second as f64) / 3.6;
            let altitude = meters(1000. + (33.333 * 33.333 - speed * speed) / (2. * GRAVITY));
            if compensated {
                let air_speed = Speed::from_meters_per_second(speed);
                add_air_speed(&mut estimator, time, air_speed);
            }
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }
        estimator
    }

    #[test]
    fn pull_up_trades_airspeed_for_altitude_without_a_climb() {
        let estimate = pull_up(true).estimate();
        let vario = assert_some!(estimate.vario);
        let raw_vertical_speed = assert_some!(estimate.raw_vertical_speed);
        let vertical_speed = assert_some!(estimate.vertical_speed);
        let expected_raw = Speed::from_meters_per_second(0.484);
        let expected_smoothed = Speed::from_meters_per_second(0.508);

        assert_abs_diff_eq!(vario, Speed::ZERO, epsilon = 0.01);
        assert_abs_diff_eq!(raw_vertical_speed, expected_raw, epsilon = 0.01);
        assert_abs_diff_eq!(vertical_speed, expected_smoothed, epsilon = 0.01);
    }

    #[test]
    fn pull_up_without_airspeed_has_no_vario_estimate() {
        let estimate = pull_up(false).estimate();
        let raw_vertical_speed = assert_some!(estimate.raw_vertical_speed);
        let vertical_speed = assert_some!(estimate.vertical_speed);
        let expected_raw = Speed::from_meters_per_second(0.484);
        let expected_smoothed = Speed::from_meters_per_second(0.508);

        assert_none!(estimate.vario);
        assert_abs_diff_eq!(raw_vertical_speed, expected_raw, epsilon = 0.01);
        assert_abs_diff_eq!(vertical_speed, expected_smoothed, epsilon = 0.01);
    }

    #[test]
    fn cleared_airspeed_removes_the_vario_estimate() {
        let mut estimator = Estimator::new();
        let air_speed = Speed::from_kilometers_per_hour(120.);
        let altitude = meters(1000.);
        for second in 0..60 {
            let time = Duration::from_secs(second);
            add_air_speed(&mut estimator, time, air_speed);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }

        estimator.clear_air_speed();
        assert_none!(estimator.estimate().vario);
        for second in 60..70 {
            let time = Duration::from_secs(second);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }

        let estimate = estimator.estimate();
        let raw_vertical_speed = assert_some!(estimate.raw_vertical_speed);
        assert_none!(estimate.vario);
        assert_abs_diff_eq!(raw_vertical_speed, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn returning_airspeed_restarts_the_vario_series() {
        let mut estimator = Estimator::new();
        let altitude = meters(1_000.);
        let air_speed = Speed::from_meters_per_second(50.);
        add_air_speed(&mut estimator, Duration::ZERO, air_speed);
        let acceptance = estimator.pressure_altitude(Duration::ZERO, altitude);
        assert_eq!(acceptance, Accepted);
        let time = Duration::from_secs(1);
        let acceptance = estimator.pressure_altitude(time, altitude);
        assert_eq!(acceptance, Accepted);

        estimator.clear_air_speed();
        let altitude = meters(1_100.);
        let time = Duration::from_secs(2);
        let acceptance = estimator.pressure_altitude(time, altitude);
        assert_eq!(acceptance, Accepted);
        let time = Duration::from_secs(3);
        let air_speed = Speed::from_meters_per_second(100.);
        add_air_speed(&mut estimator, time, air_speed);
        let acceptance = estimator.pressure_altitude(time, altitude);
        assert_eq!(acceptance, Accepted);
        assert_none!(estimator.estimate().vario);

        let time = Duration::from_secs(4);
        let acceptance = estimator.pressure_altitude(time, altitude);
        assert_eq!(acceptance, Accepted);

        let vario = assert_some!(estimator.estimate().vario);
        assert_abs_diff_eq!(vario, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn new_airspeed_does_not_change_vertical_speed() {
        let mut estimator = Estimator::new();
        let altitude = meters(1000.);
        for second in 0..60u64 {
            let time = Duration::from_secs(second);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }
        let before = assert_some!(estimator.estimate().vertical_speed);
        let air_speed = Speed::from_kilometers_per_hour(120.);
        for second in 60..70u64 {
            let time = Duration::from_secs(second);
            add_air_speed(&mut estimator, time, air_speed);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }

        let estimate = estimator.estimate();
        let vertical_speed = assert_some!(estimate.vertical_speed);
        let vario = assert_some!(estimate.vario);
        assert_eq!(vertical_speed, before);
        assert_abs_diff_eq!(vario, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn raw_vertical_speed_survives_cleared_airspeed() {
        let mut estimator = Estimator::new();
        let air_speed = Speed::from_kilometers_per_hour(120.);
        for second in 0..60u64 {
            let time = Duration::from_secs(second);
            let altitude = meters(1000. + 2. * second as f64);
            add_air_speed(&mut estimator, time, air_speed);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
        }

        estimator.clear_air_speed();
        let expected = Speed::from_meters_per_second(2.);
        for second in 60..70u64 {
            let time = Duration::from_secs(second);
            let altitude = meters(1000. + 2. * second as f64);
            let acceptance = estimator.pressure_altitude(time, altitude);
            assert_eq!(acceptance, Accepted);
            assert_none!(estimator.estimate().vario);
            let raw_vertical_speed = assert_some!(estimator.estimate().raw_vertical_speed);
            assert_abs_diff_eq!(raw_vertical_speed, expected, epsilon = 0.01);
        }
    }

    #[test]
    fn older_airspeed_preserves_the_latest_sample() {
        let mut estimator = Estimator::new();
        let air_speed = Speed::from_meters_per_second(50.);
        add_air_speed(&mut estimator, Duration::ZERO, air_speed);
        let acceptance = estimator.pressure_altitude(Duration::ZERO, meters(1_000.));
        assert_eq!(acceptance, Accepted);
        let time = Duration::from_secs(2);
        let air_speed = Speed::from_meters_per_second(40.);
        add_air_speed(&mut estimator, time, air_speed);
        let delayed_speed = Speed::from_meters_per_second(100.);
        let acceptance = estimator.air_speed(Duration::from_secs(1), delayed_speed);
        assert_eq!(acceptance, Ignored);

        let altitude_gain = (50_f64.powi(2) - 40_f64.powi(2)) / (2. * GRAVITY);
        let altitude = meters(1_000. + altitude_gain);
        let acceptance = estimator.pressure_altitude(time, altitude);
        assert_eq!(acceptance, Accepted);
        let vario = assert_some!(estimator.estimate().vario);
        assert_abs_diff_eq!(vario, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn gnss_without_barometer_produces_vertical_speed() {
        let mut estimator = Estimator::new();
        for second in 0..60u64 {
            assert_eq!(
                estimator.gnss_altitude(
                    Duration::from_secs(second),
                    EllipsoidAltitude::new(Length::from_meters(1200. + 2. * second as f64)),
                ),
                Accepted
            );
        }

        let vertical_speed = assert_some!(estimator.estimate().raw_vertical_speed);
        assert_abs_diff_eq!(
            vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn establishing_the_gnss_offset_does_not_show_as_a_climb() {
        let mut estimator = Estimator::new();
        for second in 0..60u64 {
            assert_eq!(
                estimator.pressure_altitude(Duration::from_secs(second), meters(1000.)),
                Accepted
            );
        }

        let mut worst = 0f64;
        for second in 60..90u64 {
            let time = Duration::from_secs(second);
            assert_eq!(
                estimator.gnss_altitude(time, EllipsoidAltitude::new(Length::from_meters(1200.))),
                Accepted
            );
            assert_eq!(estimator.pressure_altitude(time, meters(1000.)), Accepted);
            let vertical_speed = assert_some!(estimator.estimate().raw_vertical_speed);
            worst = worst.max(vertical_speed.as_meters_per_second().abs());
        }

        assert!(worst < 0.01, "reading reached {worst} m/s");
    }

    #[test]
    fn pressure_first_pair_publishes_gnss_referenced_altitude() {
        let mut estimator = Estimator::new();
        let position = LatLon::from_degrees(50.823, 6.186);
        estimator.position(position);
        assert_eq!(
            estimator.pressure_altitude(Duration::ZERO, meters(1_000.)),
            Accepted
        );
        assert_eq!(
            estimator.pressure_altitude(Duration::from_secs(1), meters(1_000.)),
            Accepted
        );

        let gnss = EllipsoidAltitude::new(Length::from_meters(1_200.));
        assert_eq!(
            estimator.gnss_altitude(Duration::from_secs(1), gnss),
            Accepted
        );

        let actual = assert_some!(estimator.estimate().altitude);
        let expected = updraft_egm96::ellipsoidal_to_msl(position, gnss);
        assert_abs_diff_eq!(
            actual.into_inner().as_meters(),
            expected.into_inner().as_meters(),
            epsilon = 0.01
        );
    }

    #[test]
    fn older_gnss_altitude_preserves_the_estimate_and_reference() {
        let mut estimator = Estimator::new();
        let mut control = Estimator::new();
        for second in [0, 2] {
            let time = Duration::from_secs(second);
            let altitude = EllipsoidAltitude::new(Length::from_meters(1_200. + 2. * second as f64));
            assert_eq!(estimator.gnss_altitude(time, altitude), Accepted);
            assert_eq!(control.gnss_altitude(time, altitude), Accepted);
        }
        let before = estimator.estimate();

        assert_eq!(
            estimator.gnss_altitude(
                Duration::from_secs(1),
                EllipsoidAltitude::new(Length::from_meters(10_000.)),
            ),
            Ignored
        );
        assert_eq!(estimator.estimate(), before);

        let time = Duration::from_secs(3);
        let altitude = EllipsoidAltitude::new(Length::from_meters(1_206.));
        assert_eq!(estimator.gnss_altitude(time, altitude), Accepted);
        assert_eq!(control.gnss_altitude(time, altitude), Accepted);
        assert_eq!(estimator.estimate(), control.estimate());
    }
    #[test]
    fn older_fix_preserves_the_wind_estimate_and_reference() {
        let mut estimator = Estimator::new();
        let air_speed = Speed::from_meters_per_second(30.);
        add_air_speed(&mut estimator, Duration::ZERO, air_speed);
        assert_eq!(estimator.fix(Duration::ZERO, &fix(0., 35.)), FixWithWind);
        assert_eq!(
            estimator.fix(Duration::from_secs(2), &fix(0., 35.)),
            FixPredicted
        );
        let mut control = estimator.clone();
        let before = estimator.estimate();

        assert_eq!(
            estimator.fix(Duration::from_secs(1), &fix(180., 100.)),
            FixIgnored
        );
        assert_eq!(estimator.estimate(), before);

        let time = Duration::from_secs(3);
        let fix = fix(0., 35.);
        assert_eq!(estimator.fix(time, &fix), FixPredicted);
        assert_eq!(control.fix(time, &fix), FixPredicted);
        assert_eq!(estimator.estimate(), control.estimate());
    }

    #[test]
    fn one_airspeed_sample_updates_wind_once() {
        let mut estimator = Estimator::new();
        let air_speed = Speed::from_meters_per_second(30.);
        add_air_speed(&mut estimator, Duration::ZERO, air_speed);
        for sample in 0..20 {
            let time = Duration::from_millis(sample * 50);
            let track = sample as f64 * 18.;
            let expected = if sample == 0 {
                FixWithWind
            } else {
                FixPredicted
            };
            assert_eq!(estimator.fix(time, &fix(track, 30.)), expected);
        }

        assert_none!(estimator.estimate().wind);
    }

    #[test]
    fn old_airspeed_sample_does_not_update_wind() {
        let mut estimator = Estimator::new();
        let air_speed = Speed::from_meters_per_second(30.);
        add_air_speed(&mut estimator, Duration::ZERO, air_speed);
        for sample in 0..20 {
            let time = Duration::from_secs(sample + 2);
            let track = sample as f64 * 18.;
            assert_eq!(estimator.fix(time, &fix(track, 30.)), FixPredicted);
        }

        assert_none!(estimator.estimate().wind);
    }
}
