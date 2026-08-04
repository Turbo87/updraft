use crate::circling::CirclingWind;
use crate::height::HeightFilter;
use crate::smoothing_weight;
use crate::wind::{Wind, WindFilter};
use std::time::Duration;
use updraft_polar::GlidePolar;
use updraft_units::{Angle, EllipsoidAltitude, Length, PressureAltitude, Speed};

/// Standard gravity, in m/s².
const GRAVITY: f64 = 9.80665;

/// Time constant of each of the two vertical-speed smoothing stages, in
/// seconds. Fitted against the recorded vario of an LXNAV LX9070 in
/// `testdata/weglide_1141558.igc`, which reports once per second. A
/// barometer that reports faster supports a shorter time constant, and
/// therefore less lag, but choosing one needs measurements from such a
/// sensor.
const VERTICAL_SPEED_TIME_CONSTANT: f64 = 2.;

/// Time constant of the turn-rate smoothing, in seconds. It only has to
/// suppress the fix-to-fix track noise: the turn rate itself changes
/// slowly, and the sink rate reacts to it weakly.
const TURN_RATE_TIME_CONSTANT: f64 = 3.;

/// The longest gap that two altitudes can still be differentiated across.
/// A larger gap restarts the vertical speed.
const MAX_ALTITUDE_INTERVAL: Duration = Duration::from_secs(30);

/// How long an airspeed or a fix stays usable after it arrives. It covers
/// a source that reports slowly, and expires one that went away.
const MAX_AGE: Duration = Duration::from_secs(5);

/// Airspeed below which the air-relative track is meaningless, in m/s.
const MIN_AIR_SPEED: f64 = 1.;

/// Load factor the sink rate is capped at. A steeper turn than 70° of
/// bank is turbulence or track noise, not circling.
const MAX_LOAD_FACTOR: f64 = 3.;

/// A position report from a GNSS receiver.
///
/// The position itself is not needed, because track and ground speed
/// already carry the ground velocity. Deriving that velocity from
/// consecutive positions instead gives the same accuracy, so a caller
/// whose receiver reports no track and ground speed can substitute it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fix {
    /// Track over ground, clockwise from true north.
    pub track: Angle,
    /// Speed over ground.
    pub ground_speed: Speed,
    /// Altitude, where the receiver reports one. It sharpens the vertical
    /// speed, and it is the only altitude when no barometer is connected.
    pub altitude: Option<EllipsoidAltitude>,
    /// Horizontal accuracy the receiver reports for this fix.
    pub position_accuracy: Length,
}

/// What the glider and the air around it are doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirState {
    /// How fast the glider gains height. Positive means climbing.
    ///
    /// This is the total-energy vertical speed, with the height the
    /// glider trades for airspeed taken out, as long as an airspeed is
    /// available. Without one it is the plain rate of climb, which shows
    /// the height gained by pulling up as a climb.
    pub vertical_speed: Speed,
    /// Vertical speed of the air mass: the vertical speed with the
    /// glider's own sink rate added back. Positive means rising air.
    /// `None` until an airspeed is available, because the sink rate
    /// depends on it.
    pub netto: Option<Speed>,
    /// Horizontal movement of the air mass, or `None` while the wind
    /// estimate has not converged yet.
    pub wind: Option<Wind>,
}

/// Derives vertical speed, netto and wind from flight data.
///
/// Each input arrives on its own call, at whatever rate its source
/// produces it: a barometer many times per second, a GNSS receiver once,
/// an airspeed sensor only while an instrument is connected. Read the
/// result from [`state`](Self::state) whenever it is needed. The
/// estimator keeps its state between calls, so the same code serves a
/// live sensor stream and a replayed recording.
///
/// The three outputs build on each other:
///
/// 1. The **total energy height** `h + v²/2g` removes the height that
///    the glider trades against airspeed when it pushes or pulls. `h`
///    combines both altitude sources (see [`HeightFilter`]). Its
///    derivative, through two smoothing stages, is the total-energy
///    **vertical speed**.
/// 2. The **wind** comes from the difference between the ground velocity
///    and the true airspeed (see [`WindFilter`]). Without an airspeed
///    sensor it comes from the shape of a circle instead (see
///    [`CirclingWind`]), and the airspeed the other two steps need is
///    then `‖ground velocity − wind‖`.
/// 3. Subtracting the wind from the ground velocity gives the air-relative
///    track. Its rate of change is the turn rate, which fixes the bank
///    angle and therefore the load factor. The load factor and the air
///    density at the current altitude turn the glide polar into the
///    current sink rate, and the vertical speed plus that sink rate is
///    the **netto**.
#[derive(Clone, Debug)]
pub struct AirStateEstimator {
    polar: GlidePolar,
    wind: WindFilter,
    circling: CirclingWind,
    height: HeightFilter,
    /// Latest measured airspeed, in m/s.
    measured_air_speed: Option<Timed<f64>>,
    /// Latest ground velocity towards east and north, in m/s.
    ground: Option<Timed<(f64, f64)>>,
    /// Latest pressure altitude, in metres, and whether a barometer
    /// supplied it. Without one the GNSS altitude takes its place.
    altitude: Option<Timed<(f64, bool)>>,
    previous: Option<Previous>,
    /// Air-relative track at the previous fix, absent while too slow.
    previous_heading: Option<f64>,
    first_stage: f64,
    vertical_speed: Option<f64>,
    turn_rate: f64,
}

/// A value with the time it was measured at.
#[derive(Clone, Copy, Debug)]
struct Timed<T> {
    time: Duration,
    value: T,
}

impl<T> Timed<T> {
    fn fresh_at(&self, now: Duration) -> bool {
        now.checked_sub(self.time).is_some_and(|age| age <= MAX_AGE)
    }
}

/// The height the vertical speed was last differentiated from.
#[derive(Clone, Copy, Debug)]
struct Previous {
    time: Duration,
    total_energy_height: f64,
    /// What went into that height. A change means the next difference
    /// would measure the change of basis instead of a climb.
    basis: Basis,
}

/// Which terms the total energy height was built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Basis {
    /// Whether an airspeed contributed the `v²/2g` term.
    compensated: bool,
    /// Whether a barometer supplied the altitude.
    barometric: bool,
    /// Whether the GNSS offset is part of the height yet.
    fused: bool,
}

impl AirStateEstimator {
    /// Creates an estimator for a glider with the given polar. The polar
    /// must already carry the flight's mass and bugs settings, because
    /// they scale the sink rate that the netto builds on.
    pub fn new(polar: GlidePolar) -> Self {
        Self {
            polar,
            wind: WindFilter::default(),
            circling: CirclingWind::default(),
            height: HeightFilter::default(),
            measured_air_speed: None,
            ground: None,
            altitude: None,
            previous: None,
            previous_heading: None,
            first_stage: 0.,
            vertical_speed: None,
            turn_rate: 0.,
        }
    }

    /// Takes a barometric altitude against the 1013.25 hPa datum. This is
    /// what advances the vertical speed, so a faster barometer makes a
    /// quieter reading.
    pub fn pressure_altitude(&mut self, time: Duration, altitude: PressureAltitude) {
        let altitude = altitude.into_inner().as_meters();
        self.altitude = Some(Timed {
            time,
            value: (altitude, true),
        });
        let height = self.height.pressure(time.as_secs_f64(), altitude);
        self.advance_vertical_speed(time, height, true);
    }

    /// Takes a position report.
    pub fn fix(&mut self, time: Duration, fix: &Fix) {
        let (sin_track, cos_track) = fix.track.sin_cos();
        let ground_speed = fix.ground_speed.as_meters_per_second();
        let east = ground_speed * sin_track;
        let north = ground_speed * cos_track;

        self.circling.update(time.as_secs_f64(), east, north);
        let air_speed = self.measured_air_speed.filter(|speed| speed.fresh_at(time));
        if let Some(air_speed) = air_speed {
            self.wind.update(
                self.interval_since_fix(time),
                east,
                north,
                Speed::from_meters_per_second(air_speed.value),
                fix.position_accuracy,
            );
        }
        self.update_turn_rate(time, east, north);
        self.ground = Some(Timed {
            time,
            value: (east, north),
        });

        let Some(altitude) = fix.altitude.map(|a| a.into_inner().as_meters()) else {
            return;
        };
        // A barometer measures the height changes better, so a GNSS
        // altitude only moves the offset. Without one it is all there is.
        if self.barometer_is_connected(time) {
            self.height.gnss(time.as_secs_f64(), altitude);
        } else {
            self.altitude = Some(Timed {
                time,
                value: (altitude, false),
            });
            let height = self.height.pressure(time.as_secs_f64(), altitude);
            self.advance_vertical_speed(time, height, false);
        }
    }

    /// Takes a true airspeed from a connected instrument. It expires after
    /// five seconds, so the estimate falls back on the shape of a circle
    /// when the instrument goes away.
    pub fn air_speed(&mut self, time: Duration, speed: Speed) {
        self.measured_air_speed = Some(Timed {
            time,
            value: speed.as_meters_per_second(),
        });
    }

    /// The current estimate, or `None` until two altitudes have arrived
    /// close enough together to be differentiated.
    pub fn state(&self) -> Option<AirState> {
        let vertical_speed = self.vertical_speed?;
        let air_speed = self.air_speed_at(self.previous?.time);
        Some(AirState {
            vertical_speed: Speed::from_meters_per_second(vertical_speed),
            netto: air_speed.zip(self.altitude).map(|(speed, altitude)| {
                let sink_rate = self.sink_rate(altitude.value.0, speed);
                Speed::from_meters_per_second(vertical_speed + sink_rate)
            }),
            wind: self.wind_vector().map(|(east, north)| Wind {
                direction: Angle::from_radians((-east).atan2(-north)).normalized(),
                speed: Speed::from_meters_per_second(east.hypot(north)),
            }),
        })
    }

    /// The wind, from whichever estimator the connected sensors support.
    fn wind_vector(&self) -> Option<(f64, f64)> {
        let measured = self
            .measured_air_speed
            .zip(self.ground)
            .is_some_and(|(speed, ground)| speed.fresh_at(ground.time));
        match measured {
            true => self.wind.vector(),
            false => self.circling.vector(),
        }
    }

    /// The airspeed to work from: measured where an instrument reports
    /// one, otherwise what is left of the ground velocity once the wind
    /// is taken out.
    fn air_speed_at(&self, now: Duration) -> Option<f64> {
        if let Some(speed) = self.measured_air_speed.filter(|s| s.fresh_at(now)) {
            return Some(speed.value);
        }
        let ground = self.ground.filter(|ground| ground.fresh_at(now))?;
        let (east, north) = self.wind_vector()?;
        Some((ground.value.0 - east).hypot(ground.value.1 - north))
    }

    fn barometer_is_connected(&self, now: Duration) -> bool {
        self.altitude
            .is_some_and(|altitude| altitude.value.1 && altitude.fresh_at(now))
    }

    fn interval_since_fix(&self, now: Duration) -> Option<f64> {
        let ground = self.ground?;
        now.checked_sub(ground.time)
            .filter(|interval| !interval.is_zero())
            .map(|interval| interval.as_secs_f64())
    }

    /// Differentiates the total energy height and smooths the result.
    fn advance_vertical_speed(&mut self, time: Duration, height: f64, barometric: bool) {
        let air_speed = self.air_speed_at(time);
        let basis = Basis {
            compensated: air_speed.is_some(),
            barometric,
            fused: self.height.is_fused(),
        };
        let energy = air_speed.map_or(0., |speed| speed * speed / (2. * GRAVITY));
        let total_energy_height = height + energy;
        let previous = self.previous.replace(Previous {
            time,
            total_energy_height,
            basis,
        });

        let usable = previous
            .filter(|previous| previous.basis == basis)
            .and_then(|previous| {
                let interval = time.checked_sub(previous.time)?;
                (!interval.is_zero() && interval <= MAX_ALTITUDE_INTERVAL)
                    .then_some((interval.as_secs_f64(), previous.total_energy_height))
            });
        let Some((interval, previous_height)) = usable else {
            self.first_stage = 0.;
            self.vertical_speed = None;
            return;
        };

        let raw = (total_energy_height - previous_height) / interval;
        let weight = smoothing_weight(interval, VERTICAL_SPEED_TIME_CONSTANT);
        self.first_stage += weight * (raw - self.first_stage);
        let vertical_speed = self.vertical_speed.unwrap_or(0.);
        self.vertical_speed = Some(vertical_speed + weight * (self.first_stage - vertical_speed));
    }

    /// Tracks the rate of change of the air-relative track, which is what
    /// a turn coordinator measures. Without a wind estimate it falls back
    /// on the track over ground.
    fn update_turn_rate(&mut self, time: Duration, ground_east: f64, ground_north: f64) {
        let (wind_east, wind_north) = self.wind_vector().unwrap_or((0., 0.));
        let air_east = ground_east - wind_east;
        let air_north = ground_north - wind_north;
        if air_east.hypot(air_north) < MIN_AIR_SPEED {
            self.previous_heading = None;
            return;
        }

        let heading = air_east.atan2(air_north);
        let interval = self.interval_since_fix(time);
        if let Some((interval, previous)) = interval.zip(self.previous_heading) {
            let change = Angle::from_radians(heading - previous)
                .normalized_signed()
                .as_radians();
            let weight = smoothing_weight(interval, TURN_RATE_TIME_CONSTANT);
            self.turn_rate += weight * (change / interval - self.turn_rate);
        }
        self.previous_heading = Some(heading);
    }

    /// The still-air sink rate (a positive number) at the given airspeed,
    /// the current load factor and the air density, in m/s.
    ///
    /// A glide polar is quoted as equivalent airspeed against sink rate
    /// at sea level. Both axes scale with `1/√σ` at a density ratio `σ`,
    /// and both scale with `√n` in a turn that pulls a load factor `n`,
    /// so the polar is read at `v·√σ/√n` and its result is scaled back.
    fn sink_rate(&self, altitude: f64, air_speed: f64) -> f64 {
        let root_density = isa_density_ratio(altitude).sqrt();
        let load_factor = (1. + (self.turn_rate * air_speed / GRAVITY).powi(2))
            .sqrt()
            .min(MAX_LOAD_FACTOR);
        let root_load = load_factor.sqrt();

        let equivalent = Speed::from_meters_per_second(air_speed * root_density / root_load);
        self.polar.sink_rate(equivalent).as_meters_per_second() * root_load / root_density
    }
}

/// Air density at a pressure altitude, relative to sea level, following
/// the ISA troposphere model.
///
/// The model assumes the ISA temperature at that altitude. A warm day
/// makes the real air thinner than this, which understates the sink rate
/// by about 1% per 3 K of deviation.
fn isa_density_ratio(altitude: f64) -> f64 {
    /// Temperature lapse rate divided by the sea level temperature, in 1/m.
    const LAPSE_RATE: f64 = 2.255_77e-5;
    /// `g/(R·L) − 1`, the exponent that turns the temperature ratio into
    /// a density ratio.
    const EXPONENT: f64 = 4.255_88;

    (1. - LAPSE_RATE * altitude).max(0.).powf(EXPONENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some};
    use std::f64::consts::TAU;
    use updraft_polar::POLAR_STORE;

    fn polar() -> GlidePolar {
        POLAR_STORE
            .iter()
            .find(|entry| entry.name == "JS-3-18m")
            .expect("the built-in store has a JS-3-18m polar")
            .glide_polar()
    }

    fn fix(track: f64, ground_speed: f64) -> Fix {
        Fix {
            track: Angle::from_degrees(track),
            ground_speed: Speed::from_kilometers_per_hour(ground_speed),
            altitude: None,
            position_accuracy: Length::from_meters(15.),
        }
    }

    fn meters(value: f64) -> PressureAltitude {
        PressureAltitude::new(Length::from_meters(value))
    }

    /// Flies one second: an airspeed, a fix and a pressure altitude, in
    /// the order a connected instrument and a receiver produce them.
    fn second(
        estimator: &mut AirStateEstimator,
        second: u64,
        track: f64,
        ground_speed: f64,
        altitude: f64,
        air_speed: Option<f64>,
    ) {
        let time = Duration::from_secs(second);
        if let Some(air_speed) = air_speed {
            estimator.air_speed(time, Speed::from_kilometers_per_hour(air_speed));
        }
        estimator.fix(time, &fix(track, ground_speed));
        estimator.pressure_altitude(time, meters(altitude));
    }

    #[test]
    fn the_first_altitude_has_nothing_to_differentiate() {
        let mut estimator = AirStateEstimator::new(polar());

        second(&mut estimator, 0, 90., 120., 1000., Some(120.));
        assert_none!(estimator.state());
        second(&mut estimator, 1, 90., 120., 1001., Some(120.));
        assert_some!(estimator.state());
    }

    #[test]
    fn a_long_gap_restarts_the_vertical_speed() {
        let mut estimator = AirStateEstimator::new(polar());
        for step in 0..60 {
            second(
                &mut estimator,
                step,
                90.,
                120.,
                1000. + step as f64,
                Some(120.),
            );
        }

        second(&mut estimator, 120, 90., 120., 1120., Some(120.));
        assert_none!(estimator.state());
        second(&mut estimator, 121, 90., 120., 1120., Some(120.));
        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn a_steady_climb_converges_on_its_rate() {
        let mut estimator = AirStateEstimator::new(polar());
        for step in 0..60 {
            second(
                &mut estimator,
                step,
                90.,
                120.,
                1000. + 2. * step as f64,
                Some(120.),
            );
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            state.vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    /// Decelerates from 160 to 100 km/h, gaining the matching height.
    fn pull_up(estimator: &mut AirStateEstimator, with_air_speed: bool) {
        for step in 0..60 {
            let air_speed = (160. - step as f64).max(100.);
            let speed = Speed::from_kilometers_per_hour(air_speed).as_meters_per_second();
            let altitude = 1000. + (44.44 * 44.44 - speed * speed) / (2. * GRAVITY);
            second(
                estimator,
                step,
                90.,
                air_speed,
                altitude,
                with_air_speed.then_some(air_speed),
            );
        }
    }

    #[test]
    fn a_pull_up_trades_airspeed_for_height_without_a_climb() {
        let mut estimator = AirStateEstimator::new(polar());
        pull_up(&mut estimator, true);

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn without_an_airspeed_a_pull_up_reads_as_a_climb() {
        let mut estimator = AirStateEstimator::new(polar());
        pull_up(&mut estimator, false);

        // Decelerating by 1 km/h per second converts airspeed into
        // height, which now reads as a climb. The smoothing lags by about
        // four seconds, so the reading is the rate from 105 km/h.
        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            state.vertical_speed,
            Speed::from_meters_per_second(0.82),
            epsilon = 0.02
        );
    }

    #[test]
    fn netto_adds_the_sink_rate_of_the_glider() {
        let mut estimator = AirStateEstimator::new(polar());
        for step in 0..60 {
            second(&mut estimator, step, 90., 120., 1000., Some(120.));
        }

        // Level flight at 120 km/h and 1000 m sinks at 0.60 m/s, so air
        // that holds the glider level must rise at the same rate.
        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
        assert_abs_diff_eq!(
            assert_some!(state.netto),
            Speed::from_meters_per_second(0.603),
            epsilon = 0.005
        );
    }

    #[test]
    fn circling_raises_the_sink_rate_the_netto_corrects_for() {
        let mut estimator = AirStateEstimator::new(polar());
        for step in 0..120 {
            let track = 360. * step as f64 / 20.;
            second(&mut estimator, step, track, 108., 1000., Some(108.));
        }

        // 108 km/h in a 20 s circle is 44° of bank, which raises the sink
        // rate from the 0.571 m/s of the same speed with level wings.
        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            assert_some!(state.netto) - state.vertical_speed,
            Speed::from_meters_per_second(0.645),
            epsilon = 0.005
        );
    }

    /// Circles at 108 km/h through a 10 m/s wind from the north-east,
    /// with no airspeed sensor.
    fn circle_without_a_sensor(seconds: u64) -> AirStateEstimator {
        const TURN_SECONDS: f64 = 20.;
        let air_speed = Speed::from_kilometers_per_hour(108.).as_meters_per_second();
        let mut estimator = AirStateEstimator::new(polar());
        for step in 0..seconds {
            let heading = TAU * step as f64 / TURN_SECONDS;
            let east = -6. + air_speed * heading.sin();
            let north = -8. + air_speed * heading.cos();
            let time = Duration::from_secs(step);
            estimator.fix(
                time,
                &Fix {
                    track: Angle::from_radians(east.atan2(north)),
                    ground_speed: Speed::from_meters_per_second(east.hypot(north)),
                    altitude: None,
                    position_accuracy: Length::from_meters(15.),
                },
            );
            estimator.pressure_altitude(time, meters(1000.));
        }
        estimator
    }

    #[test]
    fn circling_recovers_the_wind_without_an_airspeed_sensor() {
        let state = assert_some!(circle_without_a_sensor(60).state());
        let wind = assert_some!(state.wind);

        assert_abs_diff_eq!(
            wind.speed,
            Speed::from_meters_per_second(10.),
            epsilon = 0.3
        );
        assert_abs_diff_eq!(wind.direction, Angle::from_degrees(36.87), epsilon = 0.03);
    }

    #[test]
    fn the_netto_waits_for_an_airspeed() {
        // Before a circle is complete there is no wind, so no airspeed
        // can be derived from the ground velocity, so no sink rate.
        assert_none!(assert_some!(circle_without_a_sensor(10).state()).netto);
        assert_some!(assert_some!(circle_without_a_sensor(60).state()).netto);
    }

    #[test]
    fn losing_the_airspeed_sensor_does_not_show_as_a_climb() {
        let mut estimator = AirStateEstimator::new(polar());
        for step in 0..60 {
            second(&mut estimator, step, 90., 120., 1000., Some(120.));
        }

        // The energy term is 141 m at 120 km/h, and dropping it must not
        // read as 141 m of sink. The airspeed expires five seconds after
        // the last one arrived.
        for step in 60..70 {
            second(&mut estimator, step, 90., 120., 1000., None);
        }
        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn a_faster_barometer_gives_the_same_climb_rate() {
        let mut estimator = AirStateEstimator::new(polar());
        for tenth in 0..600 {
            let time = Duration::from_millis(tenth * 100);
            if tenth % 10 == 0 {
                estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
                estimator.fix(time, &fix(90., 120.));
            }
            estimator.pressure_altitude(time, meters(1000. + 0.2 * tenth as f64));
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            state.vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn establishing_the_gnss_offset_does_not_show_as_a_climb() {
        let mut estimator = AirStateEstimator::new(polar());
        let mut worst: f64 = 0.;
        for step in 0..30 {
            let time = Duration::from_secs(step);
            estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
            estimator.fix(
                time,
                &Fix {
                    altitude: Some(EllipsoidAltitude::new(Length::from_meters(1200.))),
                    ..fix(90., 120.)
                },
            );
            estimator.pressure_altitude(time, meters(1000.));
            if let Some(state) = estimator.state() {
                worst = worst.max(state.vertical_speed.as_meters_per_second().abs());
            }
        }

        // The two altitudes differ by 200 m. Folding that into the height
        // is a change of reference, not 200 m of climb.
        assert!(worst < 0.01, "vertical speed reached {worst} m/s");
    }

    #[test]
    fn a_receiver_without_a_barometer_still_gives_a_vertical_speed() {
        let mut estimator = AirStateEstimator::new(polar());
        for step in 0..60 {
            let time = Duration::from_secs(step);
            estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
            estimator.fix(
                time,
                &Fix {
                    altitude: Some(EllipsoidAltitude::new(Length::from_meters(
                        1200. + 2. * step as f64,
                    ))),
                    ..fix(90., 120.)
                },
            );
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            state.vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn the_isa_model_matches_the_published_density_ratios() {
        assert_abs_diff_eq!(isa_density_ratio(0.), 1., epsilon = 1e-9);
        assert_abs_diff_eq!(isa_density_ratio(1000.), 0.9075, epsilon = 5e-4);
        assert_abs_diff_eq!(isa_density_ratio(3000.), 0.7422, epsilon = 5e-4);
    }
}
