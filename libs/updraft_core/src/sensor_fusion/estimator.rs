use super::altitude::AltitudeFilter;
use super::circling::{CirclingWind, Fit, FitPolicy};
use super::inferred_airspeed::InferredAirspeed;
use super::sample::{AltitudeDomain, SampleAcceptance};
use super::smoothing::smoothing_weight;
use super::vario::Vario;
use super::wind::{Wind, WindFilter};
use std::time::Duration;
use updraft_geo::LatLon;
use updraft_polar::GlidePolar;
use updraft_units::{Angle, EllipsoidAltitude, Length, MslAltitude, PressureAltitude, Speed};

const GRAVITY: f64 = 9.80665;
const MAX_WIND_AIR_SPEED_SKEW: Duration = Duration::from_secs(1);
const TURN_RATE_TIME_CONSTANT: Duration = Duration::from_secs(3);
const MIN_AIR_SPEED: Speed = Speed::from_meters_per_second(1.);
const MAX_LOAD_FACTOR: f64 = 3.;

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
    pub air_speed: Option<Speed>,
    pub heading: Option<Angle>,
    pub altitude: Option<MslAltitude>,
    pub bank_angle: Option<Angle>,
    pub netto: Option<Speed>,
}

/// Derives flight values from timestamped physical measurements.
///
/// This layer owns numerical estimator state. [`SensorFusion`](super::SensorFusion)
/// owns selected-source continuity, freshness, and protocol projection.
#[derive(Clone, Debug)]
pub struct Estimator {
    polar: Option<GlidePolar>,
    altitude: AltitudeFilter,
    pressure_altitude_current: bool,
    vario: Vario,
    uncompensated: Vario,
    measured_air_speed: Option<AirSpeedSample>,
    inferred_air_speed: InferredAirspeed,
    vario_available: bool,
    previous_energy: Option<Length>,
    gnss_time: Option<Duration>,
    position: Option<LatLon>,
    referenced_altitude: Option<EllipsoidAltitude>,
    wind: WindFilter,
    wind_air_speed_time: Option<Duration>,
    circling: CirclingWind,
    ground: Option<GroundVelocity>,
    previous_heading: Option<Angle>,
    turn_rate: Option<f64>,
    isa_altitude_sample: Option<(Duration, Length)>,
}

#[derive(Clone, Copy, Debug)]
struct GroundVelocity {
    time: Duration,
    east: Speed,
    north: Speed,
}

impl Default for Estimator {
    fn default() -> Self {
        Self::new()
    }
}

impl Estimator {
    pub fn new() -> Self {
        Self {
            polar: None,
            altitude: AltitudeFilter::default(),
            pressure_altitude_current: false,
            vario: Vario::default(),
            uncompensated: Vario::default(),
            measured_air_speed: None,
            inferred_air_speed: InferredAirspeed::default(),
            vario_available: false,
            previous_energy: None,
            gnss_time: None,
            position: None,
            referenced_altitude: None,
            wind: WindFilter::default(),
            wind_air_speed_time: None,
            circling: CirclingWind::default(),
            ground: None,
            previous_heading: None,
            turn_rate: None,
            isa_altitude_sample: None,
        }
    }

    #[cfg(test)]
    pub fn set_polar(&mut self, polar: GlidePolar) {
        self.polar = Some(polar);
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

        let pressure_altitude = altitude.into_inner();
        self.pressure_altitude_current = true;
        let altitude = self.altitude.pressure(time, pressure_altitude);
        self.referenced_altitude = self
            .altitude
            .referenced_altitude()
            .map(EllipsoidAltitude::new);
        self.isa_altitude_sample = Some((time, pressure_altitude));
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
                self.isa_altitude_sample = Some((time, altitude_value));
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
        if self.measured_air_speed.is_none() {
            self.reset_total_energy();
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
        if self.ground.is_some_and(|previous| time <= previous.time) {
            return FixAcceptance::Ignored;
        }

        let (sin_track, cos_track) = fix.track.sin_cos();
        let east = fix.ground_speed * sin_track;
        let north = fix.ground_speed * cos_track;

        if let Some(previous) = self.ground {
            self.wind.predict(time - previous.time);
        }
        let policy = if self.wind.vector().is_none() {
            FitPolicy::AllowPartialArc
        } else {
            FitPolicy::FullCircleOnly
        };
        let fit = self.circling.update(time, east, north, policy);
        let air_speed_sample = self.measured_air_speed.filter(|sample| {
            self.wind_air_speed_time != Some(sample.time)
                && sample.time.abs_diff(time) <= MAX_WIND_AIR_SPEED_SKEW
        });
        let air_speed_measurement = air_speed_sample.map_or(SampleAcceptance::Ignored, |sample| {
            let acceptance = self.wind.update(east, north, sample.speed);
            if acceptance == SampleAcceptance::Accepted {
                self.wind_air_speed_time = Some(sample.time);
            }
            acceptance
        });
        let wind_measurement = if air_speed_measurement == SampleAcceptance::Accepted {
            air_speed_measurement
        } else {
            fit.map_or(SampleAcceptance::Ignored, |fit| {
                let (east, north) = fit.wind_vector();
                if policy == FitPolicy::AllowPartialArc && matches!(fit, Fit::FullCircle(_)) {
                    self.wind = WindFilter::default();
                }
                let acceptance = self
                    .wind
                    .update_vector(east, north, fit.measurement_variance());
                if acceptance == SampleAcceptance::Accepted {
                    self.circling.accept(fit);
                }
                acceptance
            })
        };
        let acceptance = if wind_measurement == SampleAcceptance::Accepted {
            FixAcceptance::AcceptedWithWindMeasurement
        } else if air_speed_sample.is_some() || fit.is_some() {
            FixAcceptance::RejectedWindMeasurement
        } else {
            FixAcceptance::Predicted
        };
        if let Some((wind_east, wind_north)) = self.wind.vector() {
            let east = (east - wind_east).as_meters_per_second();
            let north = (north - wind_north).as_meters_per_second();
            let speed = Speed::from_meters_per_second(east.hypot(north));
            self.inferred_air_speed
                .update(time, speed, self.circling.is_turning());
        }
        self.update_turn_rate(time, east, north);
        self.ground = Some(GroundVelocity { time, east, north });
        acceptance
    }

    pub fn clear_air_speed(&mut self) {
        if self.measured_air_speed.take().is_some() {
            self.reset_total_energy();
        }
    }

    pub fn clear_inferred_air_speed(&mut self) {
        self.inferred_air_speed = InferredAirspeed::default();
    }

    fn reset_total_energy(&mut self) {
        self.vario = Vario::default();
        self.vario_available = false;
        self.previous_energy = None;
    }

    fn air_speed_at(&self, now: Duration) -> Option<Speed> {
        self.measured_air_speed
            .map(|sample| sample.speed)
            .or_else(|| self.inferred_air_speed.fresh_at(now))
    }

    fn update_turn_rate(&mut self, time: Duration, ground_east: Speed, ground_north: Speed) {
        let Some((wind_east, wind_north)) = self.wind.vector() else {
            self.previous_heading = None;
            self.turn_rate = None;
            return;
        };
        let air_east = (ground_east - wind_east).as_meters_per_second();
        let air_north = (ground_north - wind_north).as_meters_per_second();
        let air_speed = Speed::from_meters_per_second(air_east.hypot(air_north));
        if air_speed < MIN_AIR_SPEED {
            self.previous_heading = None;
            self.turn_rate = None;
            return;
        }

        let heading = Angle::from_radians(air_east.atan2(air_north));
        let interval = self.ground.map(|previous| time - previous.time);
        if let Some((interval, previous)) = interval.zip(self.previous_heading) {
            let change = (heading - previous).normalized_signed().as_radians();
            let weight = smoothing_weight(interval, TURN_RATE_TIME_CONSTANT);
            let measured_turn_rate = change / interval.as_secs_f64();
            let turn_rate = self.turn_rate.unwrap_or(0.);
            self.turn_rate = Some(turn_rate + weight * (measured_turn_rate - turn_rate));
        }
        self.previous_heading = Some(heading);
    }

    fn bank_angle(&self, air_speed: Speed) -> Option<Angle> {
        let turn_rate = self.turn_rate?;
        let limit = (MAX_LOAD_FACTOR * MAX_LOAD_FACTOR - 1.).sqrt();
        let tangent = (turn_rate * air_speed.as_meters_per_second() / GRAVITY).clamp(-limit, limit);
        Some(Angle::from_radians(tangent.atan()))
    }

    fn sink_rate(&self, altitude: Length, air_speed: Speed) -> Option<Speed> {
        let polar = self.polar.as_ref()?;
        let density = isa_density_ratio(altitude);
        let root_density = density.sqrt();
        let bank_angle = self.bank_angle(air_speed).unwrap_or(Angle::ZERO);
        let load = 1. / bank_angle.cos();
        let root_load = load.sqrt();
        let equivalent = air_speed * root_density / root_load;
        Some(polar.sink_rate(equivalent) * load * root_load / root_density)
    }

    fn heading(&self) -> Option<Angle> {
        let ground = self.ground?;
        let (wind_east, wind_north) = self.wind.vector()?;
        let east = (ground.east - wind_east).as_meters_per_second();
        let north = (ground.north - wind_north).as_meters_per_second();
        let speed = Speed::from_meters_per_second(east.hypot(north));
        (speed >= MIN_AIR_SPEED).then(|| Angle::from_radians(east.atan2(north)).normalized())
    }

    /// Clears state that assumes continuity with one airspeed source.
    pub fn reset_air_speed(&mut self) {
        self.measured_air_speed = None;
        self.reset_total_energy();
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
        let air_speed = self.air_speed_at(time);
        self.vario_available = air_speed.is_some();
        let energy = air_speed.map_or(Length::ZERO, |speed| {
            let speed = speed.as_meters_per_second();
            Length::from_meters(speed * speed / (2. * GRAVITY))
        });
        let compensation = match (self.previous_energy, air_speed.is_some()) {
            (None, true) => energy,
            (Some(previous), false) => -previous,
            _ => Length::ZERO,
        };
        let fusion = self.altitude.take_step();
        self.previous_energy = air_speed.map(|_| energy);
        let compensated =
            self.vario
                .advance(time, altitude + energy, fusion + compensation, domain);
        let uncompensated = self.uncompensated.advance(time, altitude, fusion, domain);
        debug_assert_eq!(compensated, acceptance);
        debug_assert_eq!(uncompensated, acceptance);
        acceptance
    }

    /// Clears state that assumes continuity with one altitude source.
    pub fn reset_altitude(&mut self) {
        self.altitude = AltitudeFilter::default();
        self.pressure_altitude_current = false;
        self.vario = Vario::default();
        self.uncompensated = Vario::default();
        self.previous_energy = None;
        self.gnss_time = None;
        self.referenced_altitude = None;
        self.vario_available = false;
        self.isa_altitude_sample = None;
    }

    pub fn reset_wind(&mut self) {
        self.wind = WindFilter::default();
        self.wind_air_speed_time = None;
        self.circling = CirclingWind::default();
        self.inferred_air_speed = InferredAirspeed::default();
        self.ground = None;
        self.previous_heading = None;
        self.turn_rate = None;
    }

    pub fn estimate(&self) -> Estimate {
        let raw_vertical_speed = self.uncompensated.value();
        let vertical_speed = self.uncompensated.smoothed_value();
        let vario = if self.vario_available {
            self.vario.smoothed_value()
        } else {
            None
        };
        let wind = self.wind.vector().map(|(east, north)| {
            let east = east.as_meters_per_second();
            let north = north.as_meters_per_second();
            Wind {
                direction: Angle::from_radians((-east).atan2(-north)).normalized(),
                speed: Speed::from_meters_per_second(east.hypot(north)),
            }
        });
        let air_speed = self
            .measured_air_speed
            .map(|sample| sample.speed)
            .or_else(|| self.inferred_air_speed.current_raw_at(self.ground?.time));
        Estimate {
            raw_vertical_speed,
            vertical_speed,
            vario,
            wind,
            air_speed,
            heading: self.heading(),
            altitude: self.altitude_msl(),
            bank_angle: air_speed.and_then(|speed| self.bank_angle(speed)),
            netto: self.isa_altitude_sample.and_then(|(time, altitude)| {
                let vertical_speed = vario?;
                let air_speed = self.air_speed_at(time)?;
                let sink_rate = self.sink_rate(altitude, air_speed)?;
                Some(vertical_speed + sink_rate)
            }),
        }
    }
}

fn isa_density_ratio(altitude: Length) -> f64 {
    const LAPSE_RATE: f64 = 2.255_77e-5;
    const EXPONENT: f64 = 4.255_88;

    let temperature_ratio = (1. - LAPSE_RATE * altitude.as_meters()).max(0.);
    temperature_ratio.powf(EXPONENT)
}

#[cfg(test)]
mod tests {
    use super::FixAcceptance::{
        AcceptedWithWindMeasurement as FixWithWind, Ignored as FixIgnored,
        Predicted as FixPredicted, RejectedWindMeasurement as FixRejectedWind,
    };
    use super::SampleAcceptance::{Accepted, Ignored};
    use super::*;
    use crate::sensor_fusion::circling::FULL_CIRCLE_VARIANCE;
    use approx::assert_abs_diff_eq;
    use claims::{assert_lt, assert_none, assert_some};
    use updraft_units::{Length, Mass};

    const GLIDER_TYPE: &str = "JS-3-18m";
    const LEVEL_SPEED: Speed = Speed::from_kilometers_per_hour(108.);

    fn polar() -> GlidePolar {
        updraft_polar::POLAR_STORE
            .iter()
            .find(|entry| entry.name == GLIDER_TYPE)
            .expect("the built-in store has this glider type")
            .glide_polar()
    }

    fn estimator_with_polar(polar: GlidePolar) -> Estimator {
        let mut estimator = Estimator::new();
        estimator.set_polar(polar);
        estimator
    }

    fn fix_kph(track: f64, ground_speed: f64) -> Fix {
        Fix {
            track: Angle::from_degrees(track),
            ground_speed: Speed::from_kilometers_per_hour(ground_speed),
        }
    }

    fn applied_sink(speed: f64, circle: Option<f64>) -> f64 {
        let mut estimator = estimator_with_polar(polar());
        for step in 0..120u64 {
            let time = Duration::from_secs(step);
            let track = circle.map_or(90., |circle| 360. * step as f64 / circle);
            add_air_speed(&mut estimator, time, Speed::from_kilometers_per_hour(speed));
            let _ = estimator.fix(time, &fix_kph(track, speed));
            assert_eq!(estimator.pressure_altitude(time, meters(1000.)), Accepted);
        }
        let state = estimator.estimate();
        (assert_some!(state.netto) - assert_some!(state.vario)).as_meters_per_second()
    }

    fn fly_level(estimator: &mut Estimator, air_speed: Option<Speed>) {
        for second in 0..60u64 {
            let time = Duration::from_secs(second);
            if let Some(speed) = air_speed {
                add_air_speed(estimator, time, speed);
                let _ = estimator.fix(time, &fix_kph(90., speed.as_kilometers_per_hour()));
            }
            assert_eq!(estimator.pressure_altitude(time, meters(1000.)), Accepted);
        }
    }

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

    fn velocity_fix(east: f64, north: f64) -> Fix {
        fix(east.atan2(north).to_degrees(), east.hypot(north))
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

    fn circle_without_air_speed(seconds: u64) -> Estimator {
        let mut estimator = Estimator::new();
        for second in 0..seconds {
            let heading = std::f64::consts::TAU * second as f64 / 20.;
            let fix = velocity_fix(-6. + 30. * heading.sin(), -8. + 30. * heading.cos());
            let _ = estimator.fix(Duration::from_secs(second), &fix);
        }
        estimator
    }

    fn circling(turn_seconds: f64, speed: Speed) -> Estimator {
        let mut estimator = Estimator::new();
        for second in 0..120 {
            let time = Duration::from_secs(second);
            let track = 360. * second as f64 / turn_seconds;
            add_air_speed(&mut estimator, time, speed);
            let _ = estimator.fix(time, &fix(track, speed.as_meters_per_second()));
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
    fn old_airspeed_sample_falls_back_to_partial_arc_measurements() {
        let mut estimator = Estimator::new();
        let air_speed = Speed::from_meters_per_second(30.);
        add_air_speed(&mut estimator, Duration::ZERO, air_speed);
        for sample in 0..20 {
            let time = Duration::from_secs(sample + 2);
            let track = sample as f64 * 18.;
            let expected = if [7, 12, 17].contains(&sample) {
                FixWithWind
            } else {
                FixPredicted
            };
            assert_eq!(estimator.fix(time, &fix(track, 30.)), expected);
        }

        assert_none!(estimator.estimate().wind);
    }

    #[test]
    fn rejected_airspeed_uses_circle_fit_measurements() {
        let mut estimator = Estimator::new();
        let mut measurements = Vec::new();
        for second in 0..=20 {
            let time = Duration::from_secs(second);
            let heading = std::f64::consts::TAU * second as f64 / 20.;
            add_air_speed(&mut estimator, time, Speed::ZERO);
            let fix = velocity_fix(-6. + 30. * heading.sin(), -8. + 30. * heading.cos());
            match estimator.fix(time, &fix) {
                FixWithWind => measurements.push(second),
                FixRejectedWind => {}
                acceptance => panic!("unexpected acceptance at second {second}: {acceptance:?}"),
            }
        }

        assert_eq!(measurements, [7, 14, 20]);
        assert_some!(estimator.estimate().wind);
    }

    #[test]
    fn missing_wind_accepts_a_partial_arc_without_publishing_motion() {
        let mut estimator = Estimator::new();
        for sample in 0..8 {
            let time = Duration::from_secs(sample);
            let heading = std::f64::consts::FRAC_PI_2 * sample as f64 / 7.;
            let fix = velocity_fix(30. * heading.sin(), 30. * heading.cos());
            let expected = if sample == 7 {
                FixWithWind
            } else {
                FixPredicted
            };
            assert_eq!(estimator.fix(time, &fix), expected);
        }

        let estimate = estimator.estimate();
        assert_none!(estimate.wind);
        assert_none!(estimate.air_speed);
        assert_none!(estimate.heading);
    }

    #[test]
    fn repeated_partial_arcs_recover_motion() {
        let mut estimator = Estimator::new();
        let mut time = 0;
        let mut measurements = 0;
        for arc in 0..6 {
            let first_sample = if arc == 0 { 0 } else { 1 };
            for sample in first_sample..8 {
                let progress = sample as f64 / 7.;
                let heading = if arc % 2 == 0 {
                    std::f64::consts::FRAC_PI_2 * progress
                } else {
                    std::f64::consts::FRAC_PI_2 * (1. - progress)
                };
                let fix = velocity_fix(30. * heading.sin(), 30. * heading.cos());
                if estimator.fix(Duration::from_secs(time), &fix) == FixWithWind {
                    measurements += 1;
                }
                time += 1;
            }
        }

        assert_eq!(measurements, 6);
        let estimate = estimator.estimate();
        assert_some!(estimate.wind);
        assert_some!(estimate.air_speed);
        assert_some!(estimate.heading);
    }

    #[test]
    fn full_circle_replaces_partial_arc_confidence() {
        let mut estimator = Estimator::new();
        let mut measurements = Vec::new();
        for second in 0..=20 {
            let heading = std::f64::consts::TAU * second as f64 / 20.;
            let fix = velocity_fix(30. * heading.sin(), 30. * heading.cos());
            if estimator.fix(Duration::from_secs(second), &fix) == FixWithWind {
                measurements.push(second);
            }
        }
        assert_eq!(measurements, [7, 12, 17, 20]);
        assert_some!(estimator.estimate().wind);

        for second in 21..=80 {
            assert_eq!(
                estimator.fix(Duration::from_secs(second), &velocity_fix(0., 30.)),
                FixPredicted
            );
        }
        assert_none!(estimator.estimate().wind);
    }

    #[test]
    fn reported_wind_suppresses_partial_arc_measurements() {
        let mut estimator = Estimator::new();
        assert_eq!(
            estimator.wind.update_vector(
                Speed::from_meters_per_second(-6.),
                Speed::from_meters_per_second(-8.),
                FULL_CIRCLE_VARIANCE,
            ),
            Accepted
        );
        assert_some!(estimator.estimate().wind);

        for sample in 0..8 {
            let time = Duration::from_secs(sample);
            let heading = std::f64::consts::FRAC_PI_2 * sample as f64 / 7.;
            let fix = velocity_fix(-6. + 30. * heading.sin(), -8. + 30. * heading.cos());
            assert_eq!(estimator.fix(time, &fix), FixPredicted);
        }
    }

    #[test]
    fn unused_circle_measurement_remains_available() {
        let mut estimator = Estimator::new();
        for second in 0..=20 {
            let time = Duration::from_secs(second);
            let heading = std::f64::consts::TAU * second as f64 / 20.;
            add_air_speed(&mut estimator, time, Speed::from_meters_per_second(30.));
            let fix = velocity_fix(-6. + 30. * heading.sin(), -8. + 30. * heading.cos());
            assert_eq!(estimator.fix(time, &fix), FixWithWind);
        }

        estimator.clear_air_speed();
        let time = Duration::from_secs(21);
        let heading = std::f64::consts::TAU * 21. / 20.;
        let fix = velocity_fix(-6. + 30. * heading.sin(), -8. + 30. * heading.cos());

        assert_eq!(estimator.fix(time, &fix), FixWithWind);
    }

    #[test]
    fn circling_without_a_sensor_infers_airspeed() {
        let estimator = circle_without_air_speed(60);

        assert_abs_diff_eq!(
            assert_some!(estimator.estimate().air_speed),
            Speed::from_meters_per_second(30.),
            epsilon = 0.3
        );
    }

    #[test]
    fn circling_without_a_sensor_infers_heading() {
        let estimator = circle_without_air_speed(60);

        assert_abs_diff_eq!(
            assert_some!(estimator.estimate().heading),
            Angle::from_degrees(342.),
            epsilon = 0.5
        );
    }

    #[test]
    fn reset_wind_restarts_the_turn_rate() {
        let mut estimator = circling(20., Speed::from_meters_per_second(30.));
        let bank = assert_some!(estimator.estimate().bank_angle);
        assert!(bank.as_degrees().abs() > 1.);

        estimator.reset_wind();
        assert_eq!(
            estimator.fix(Duration::from_secs(121), &fix(0., 35.)),
            FixPredicted
        );

        assert_none!(estimator.estimate().bank_angle);
    }

    #[test]
    fn turn_is_reported_as_bank_angle() {
        let estimator = circling(20., Speed::from_meters_per_second(30.));

        assert_abs_diff_eq!(
            assert_some!(estimator.estimate().bank_angle).as_degrees(),
            43.86,
            epsilon = 0.05
        );
    }

    #[test]
    fn turn_direction_changes_bank_direction() {
        let speed = Speed::from_meters_per_second(30.);
        let right = assert_some!(circling(20., speed).estimate().bank_angle);
        let left = assert_some!(circling(-20., speed).estimate().bank_angle);

        assert_abs_diff_eq!(right.as_degrees(), -left.as_degrees(), epsilon = 0.05);
    }

    #[test]
    fn extreme_turn_rate_stops_at_load_factor_limit() {
        let estimator = circling(4., Speed::from_meters_per_second(30.));

        assert_abs_diff_eq!(
            assert_some!(estimator.estimate().bank_angle).as_degrees(),
            70.53,
            epsilon = 0.05
        );
    }

    #[test]
    fn netto_adds_the_glider_sink_rate() {
        let mut estimator = estimator_with_polar(polar());
        fly_level(&mut estimator, Some(LEVEL_SPEED));

        let state = estimator.estimate();
        let expected = Speed::from_meters_per_second(0.571);
        assert_abs_diff_eq!(assert_some!(state.vario), Speed::ZERO, epsilon = 0.01);
        assert_abs_diff_eq!(assert_some!(state.netto), expected, epsilon = 0.01);
    }

    #[test]
    fn netto_uses_pressure_altitude_for_density() {
        fn level_flight(gnss_offset: Option<Length>) -> Speed {
            let mut estimator = estimator_with_polar(polar());
            for step in 0..60u64 {
                let time = Duration::from_secs(step);
                add_air_speed(&mut estimator, time, LEVEL_SPEED);
                let _ = estimator.fix(time, &fix_kph(90., 108.));
                assert_eq!(estimator.pressure_altitude(time, meters(1000.)), Accepted);
                if let Some(offset) = gnss_offset {
                    let altitude = EllipsoidAltitude::new(Length::from_meters(1000.) + offset);
                    assert_eq!(estimator.gnss_altitude(time, altitude), Accepted);
                }
            }
            assert_some!(estimator.estimate().netto)
        }

        let pressure_only = level_flight(None);
        let gnss_referenced = level_flight(Some(Length::from_meters(1000.)));

        assert_abs_diff_eq!(gnss_referenced, pressure_only, epsilon = 1e-9);
    }

    #[test]
    fn netto_requires_a_polar_and_airspeed() {
        let mut without_polar = Estimator::new();
        let mut without_airspeed = estimator_with_polar(polar());
        fly_level(&mut without_polar, Some(LEVEL_SPEED));
        fly_level(&mut without_airspeed, None);

        assert_none!(without_polar.estimate().netto);
        assert_none!(without_airspeed.estimate().netto);
    }

    #[test]
    fn dumping_water_ballast_lightens_the_sink_rate_at_once() {
        let heavy = polar().with_total_mass(Mass::from_kilograms(600.));
        let light = polar().with_total_mass(Mass::from_kilograms(400.));

        let mut estimator = estimator_with_polar(heavy);
        fly_level(&mut estimator, Some(LEVEL_SPEED));
        let ballasted = assert_some!(estimator.estimate().netto);

        estimator.set_polar(light);
        let dumped = assert_some!(estimator.estimate().netto);

        assert_lt!(dumped, ballasted);
    }

    #[test]
    fn turning_sink_rate_follows_the_load_factor_law() {
        const CIRCLE: f64 = 20.;
        const SPEED: f64 = 108.;

        let turn_rate = std::f64::consts::TAU / CIRCLE;
        let load = (1. + (turn_rate * SPEED / 3.6 / GRAVITY).powi(2)).sqrt();
        let turning = applied_sink(SPEED, Some(CIRCLE));
        let level = applied_sink(SPEED / load.sqrt(), None);

        assert_abs_diff_eq!(turning / level, load * load.sqrt(), epsilon = 0.01);
    }

    #[test]
    fn the_isa_model_matches_the_published_density_ratios() {
        let at_1000m = isa_density_ratio(Length::from_meters(1000.));
        let at_3000m = isa_density_ratio(Length::from_meters(3000.));
        let at_5000m = isa_density_ratio(Length::from_meters(5000.));

        assert_abs_diff_eq!(isa_density_ratio(Length::ZERO), 1., epsilon = 1e-9);
        assert_abs_diff_eq!(at_1000m, 0.9075, epsilon = 0.0005);
        assert_abs_diff_eq!(at_3000m, 0.7423, epsilon = 0.0005);
        assert_abs_diff_eq!(at_5000m, 0.6012, epsilon = 0.0005);
    }

    #[test]
    fn bank_angle_waits_for_air_speed() {
        assert_none!(circle_without_air_speed(10).estimate().bank_angle);
        assert_some!(circle_without_air_speed(60).estimate().bank_angle);
    }

    #[test]
    fn bank_angle_waits_for_wind() {
        let mut estimator = Estimator::new();
        let speed = Speed::from_meters_per_second(30.);
        for second in 0..3 {
            let time = Duration::from_secs(second);
            add_air_speed(&mut estimator, time, speed);
            let _ = estimator.fix(time, &fix(18. * second as f64, 30.));
        }

        assert_none!(estimator.estimate().bank_angle);
    }
}
