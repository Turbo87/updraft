use crate::circling::{self, CirclingWind};
use crate::height::HeightFilter;
use crate::noise::NoiseFloor;
use crate::smoothing_weight;
use crate::wind::{Wind, WindFilter};
use std::time::Duration;
use updraft_geo::LatLon;
use updraft_units::{Angle, EllipsoidAltitude, Length, MslAltitude, PressureAltitude, Speed};

/// Standard gravity, in m/s².
const GRAVITY: f64 = 9.80665;

/// Default time constant of each of the two vertical-speed smoothing
/// stages, in seconds. Fitted against the recorded vario of an LXNAV
/// LX9070.
///
/// It suits a pressure altitude that carries about 0.6 m of noise, which
/// is what a flight recorder logs. A quieter sensor supports a shorter
/// one; see
/// [`with_vertical_speed_time_constant`](AirStateEstimator::with_vertical_speed_time_constant).
const DEFAULT_VERTICAL_SPEED_TIME_CONSTANT: f64 = 2.;

/// The noise that [`DEFAULT_VERTICAL_SPEED_TIME_CONSTANT`] was fitted at,
/// in metres.
const REFERENCE_NOISE: f64 = 0.6;

/// How the time constant follows the noise of the source.
///
/// It is a two-point calibration. A logged pressure altitude carries
/// 0.6 m and needs 2 s. The barometer of a Galaxy S23 carries 0.024 m and
/// holds the same vertical-speed noise at 0.25 s. A power law through
/// those two points has this exponent.
const NOISE_EXPONENT: f64 = 0.65;

/// Time constants the measurement is allowed to choose, in seconds.
const MIN_TIME_CONSTANT: f64 = 0.25;
const MAX_TIME_CONSTANT: f64 = 3.;

/// The longest gap that two altitudes can still be differentiated across.
/// A larger gap restarts the vertical speed.
const MAX_ALTITUDE_INTERVAL: Duration = Duration::from_secs(30);

/// Airspeed below which the air-relative track is meaningless, in m/s.
const MIN_AIR_SPEED: f64 = 1.;

/// How long an airspeed stays usable after it arrives. It covers a
/// source that reports slowly, and expires one that went away.
const MAX_AGE: Duration = Duration::from_secs(5);

/// A position and ground velocity from a GNSS receiver.
///
/// The altitude arrives on its own call. A receiver reports the two
/// together, but NMEA splits them across `RMC` and `GGA`, and each has
/// to keep its own time: the height filter pairs a GNSS altitude with a
/// pressure altitude within 0.2 s, and one second of a climb put into
/// that pair is two metres of error.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fix {
    /// Where the glider is.
    pub position: LatLon,
    /// Track over ground, clockwise from true north.
    pub track: Angle,
    /// Speed over ground.
    pub ground_speed: Speed,
    /// Horizontal accuracy the receiver reports for this fix.
    pub position_accuracy: Length,
}

/// What the glider and the air around it are doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirState {
    /// How fast the glider gains energy. Positive means climbing.
    ///
    /// This is the total-energy vertical speed, with the height that the
    /// glider trades for airspeed taken out. Without an airspeed it is
    /// the rate of climb itself.
    pub vertical_speed: Speed,
    /// How fast the glider gains height, with no total-energy
    /// compensation. Positive means climbing.
    ///
    /// This is what a vertical speed indicator shows. It counts the
    /// height gained by pulling up as a climb, which is what a pilot
    /// wants under power, and it stays available when the airspeed goes
    /// away.
    pub rate_of_climb: Speed,
    /// True airspeed. It comes from a connected instrument where one
    /// reports it, and otherwise from the ground velocity with the wind
    /// taken out, which needs the wind estimate to have converged.
    ///
    /// The derived value uses the horizontal velocities alone, so a
    /// steep climb understates it by a few tenths of a percent.
    pub air_speed: Option<Speed>,
    /// The direction the glider points, clockwise from true north.
    ///
    /// This is the air-relative track, which a coordinated turn keeps
    /// aligned with the fuselage. `None` until the wind is known,
    /// because without the wind it would only repeat the track over
    /// ground that the caller supplied.
    pub heading: Option<Angle>,
    /// Horizontal movement of the air mass, or `None` while the wind
    /// estimate has not converged yet.
    pub wind: Option<Wind>,
    /// Height above mean sea level, from both altitude sources. `None`
    /// until a GNSS altitude has arrived, because a pressure altitude
    /// alone has no sea-level reference.
    pub altitude: Option<MslAltitude>,
}

/// Derives the [`AirState`] from flight data.
///
/// Each input arrives on its own call, at whatever rate its source
/// produces it. Read the result from [`state`](Self::state) whenever it
/// is needed. The estimator keeps its state between calls, so the same
/// code serves a live sensor stream and a replayed recording.
///
/// A sample that is not a finite number is ignored where it arrives. The
/// filters hold their state for the whole flight and none of them can
/// heal, because a comparison against a NaN is false and every guard
/// that would reject it therefore passes.
#[derive(Clone, Debug)]
pub struct AirStateEstimator {
    /// Combines the two altitude sources into one height.
    height: HeightFilter,
    /// Latest pressure altitude, in metres, and whether a barometer
    /// supplied it. Without one the GNSS altitude takes its place.
    altitude: Option<Timed<(f64, bool)>>,
    /// Vertical speed of the total energy height.
    total_energy: Vario,
    /// Vertical speed of the height alone, without the energy term.
    uncompensated: Vario,
    /// Latest airspeed from an instrument.
    measured_air_speed: Option<Timed<f64>>,
    /// The airspeed term of the previous total energy height, absent
    /// where no airspeed contributed one.
    previous_energy: Option<f64>,
    /// Estimates the wind from the ground velocity and the airspeed.
    wind: WindFilter,
    /// Measures the wind from the shape of a circle instead.
    circling: CirclingWind,
    /// Latest ground velocity, east and north in m/s.
    ground: Option<Timed<(f64, f64)>>,
    /// Latest position, for the geoid.
    position: Option<LatLon>,
    /// Latest fused height, and whether it is referenced to the
    /// ellipsoid rather than to the pressure datum.
    height_value: Option<(f64, bool)>,
    /// Noise and resolution of the source that drives the height.
    noise: NoiseFloor,
    /// Set by the caller, or `None` to follow the measured noise.
    vertical_speed_time_constant: Option<f64>,
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

/// The height a vertical speed was last differentiated from.
#[derive(Clone, Copy, Debug)]
struct Previous {
    time: Duration,
    height: f64,
    /// Whether a barometer supplied that height. A barometer and a GNSS
    /// receiver measure against datums that are hundreds of metres
    /// apart, and nothing here knows the distance between them.
    barometric: bool,
}

/// Differentiates a height series and smooths the result through two
/// exponential stages.
#[derive(Clone, Debug, Default)]
struct Vario {
    previous: Option<Previous>,
    first_stage: f64,
    value: Option<f64>,
}

impl Vario {
    /// Takes the next height.
    ///
    /// `rebase` is how far the height moved because of a change of
    /// reference rather than a climb: the airspeed term appearing or
    /// going away. It is added to the stored previous height, so the
    /// difference stays a climb and the filter keeps running.
    ///
    /// A change of datum, or a gap longer than [`MAX_ALTITUDE_INTERVAL`],
    /// still restarts the filter. Neither carries a known step.
    fn advance(
        &mut self,
        time: Duration,
        height: f64,
        rebase: f64,
        barometric: bool,
        time_constant: f64,
    ) {
        if let Some(previous) = self.previous.as_mut() {
            previous.height += rebase;
        }

        // A second sample at a time that has not advanced carries
        // nothing to differentiate, and it must not become the reference
        // either: the next real sample would be measured from it and the
        // reading would restart. Two devices that both report a pressure
        // altitude arrive in one batch under one timestamp, so this is
        // the ordinary case and not a fault.
        let repeated = self
            .previous
            .is_some_and(|previous| previous.barometric == barometric && time <= previous.time);
        if repeated {
            return;
        }

        let previous = self.previous.replace(Previous {
            time,
            height,
            barometric,
        });

        let usable = previous
            .filter(|previous| previous.barometric == barometric)
            .and_then(|previous| {
                let interval = time.checked_sub(previous.time)?;
                (!interval.is_zero() && interval <= MAX_ALTITUDE_INTERVAL)
                    .then_some((interval.as_secs_f64(), previous.height))
            });
        let Some((interval, previous_height)) = usable else {
            self.first_stage = 0.;
            self.value = None;
            return;
        };

        let raw = (height - previous_height) / interval;
        let weight = smoothing_weight(interval, time_constant);
        self.first_stage += weight * (raw - self.first_stage);
        let value = self.value.unwrap_or(0.);
        self.value = Some(value + weight * (self.first_stage - value));
    }
}

impl Default for AirStateEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl AirStateEstimator {
    pub fn new() -> Self {
        Self {
            height: HeightFilter::default(),
            altitude: None,
            total_energy: Vario::default(),
            uncompensated: Vario::default(),
            measured_air_speed: None,
            previous_energy: None,
            wind: WindFilter::default(),
            circling: CirclingWind::default(),
            ground: None,
            position: None,
            height_value: None,
            noise: NoiseFloor::default(),
            vertical_speed_time_constant: None,
        }
    }

    /// Overrides the vertical-speed smoothing, which otherwise follows
    /// the noise of the altitude source. A caller that knows its sensor
    /// better than the measurement does can set it here.
    pub fn with_vertical_speed_time_constant(mut self, time_constant: Duration) -> Self {
        let seconds = time_constant.as_secs_f64();
        if seconds.is_finite() && seconds > 0. {
            self.vertical_speed_time_constant = Some(seconds);
        }
        self
    }

    /// How hard to smooth the vertical speed, in seconds.
    ///
    /// A noisy source needs a longer time constant and a quiet one buys
    /// a shorter reading. The measurement only shortens the default
    /// where the source resolves its own noise: a sensor quantised more
    /// coarsely than it is noisy reads a floor of zero, which says
    /// nothing about how noisy it really is.
    fn vertical_speed_time_constant(&self) -> f64 {
        if let Some(set) = self.vertical_speed_time_constant {
            return set;
        }
        let Some(noise) = self.noise.noise() else {
            return DEFAULT_VERTICAL_SPEED_TIME_CONSTANT;
        };

        let derived =
            DEFAULT_VERTICAL_SPEED_TIME_CONSTANT * (noise / REFERENCE_NOISE).powf(NOISE_EXPONENT);
        let derived = derived.clamp(MIN_TIME_CONSTANT, MAX_TIME_CONSTANT);
        match self.noise.resolves_its_own_noise() {
            true => derived,
            false => derived.max(DEFAULT_VERTICAL_SPEED_TIME_CONSTANT),
        }
    }

    /// Takes a barometric altitude against the 1013.25 hPa datum.
    pub fn pressure_altitude(&mut self, time: Duration, altitude: PressureAltitude) {
        let altitude = altitude.into_inner().as_meters();
        if !altitude.is_finite() {
            return;
        }
        self.noise.update(time.as_secs_f64(), altitude);
        self.altitude = Some(Timed {
            time,
            value: (altitude, true),
        });
        let height = self.height.pressure(time.as_secs_f64(), altitude);
        self.height_value = Some((height, self.height.is_fused()));
        self.advance_vertical_speed(time, height, true);
    }

    /// Takes a GNSS altitude against the WGS84 ellipsoid.
    ///
    /// The ellipsoid rather than mean sea level, because a receiver's own
    /// geoid model is coarse and two receivers do not share one.
    ///
    /// A barometer measures the height changes better, so this only moves
    /// the offset while one is connected. Without a barometer it is all
    /// there is, and it drives the height itself.
    pub fn gnss_altitude(&mut self, time: Duration, altitude: EllipsoidAltitude) {
        let altitude = altitude.into_inner().as_meters();
        if !altitude.is_finite() {
            return;
        }
        if self.barometer_is_connected(time) {
            self.height.gnss(time.as_secs_f64(), altitude);
            if let Some(value) = self.height_value.as_mut() {
                value.1 = self.height.is_fused();
            }
        } else {
            self.altitude = Some(Timed {
                time,
                value: (altitude, false),
            });
            // The GNSS altitude is the height itself here, so it does not
            // go through the offset. That offset is the difference
            // between the two altitudes, and this altitude already sits
            // on the GNSS side of it: adding it would count it twice.
            self.height_value = Some((altitude, true));
            self.advance_vertical_speed(time, altitude, false);
        }
    }

    /// The height above mean sea level, once the GNSS altitude has given
    /// the height a sea-level reference.
    fn msl_altitude(&self) -> Option<MslAltitude> {
        let (height, _) = self.height_value.filter(|(_, referenced)| *referenced)?;
        let ellipsoidal = EllipsoidAltitude::new(Length::from_meters(height));
        Some(updraft_egm96::ellipsoidal_to_msl(
            self.position?,
            ellipsoidal,
        ))
    }

    fn barometer_is_connected(&self, now: Duration) -> bool {
        self.altitude
            .is_some_and(|altitude| altitude.value.1 && altitude.fresh_at(now))
    }

    /// Takes a position report.
    pub fn fix(&mut self, time: Duration, fix: &Fix) {
        let finite = fix.track.as_degrees().is_finite()
            && fix.ground_speed.as_meters_per_second().is_finite()
            && fix.position_accuracy.as_meters().is_finite()
            && fix.position.latitude().as_degrees().is_finite()
            && fix.position.longitude().as_degrees().is_finite();
        if !finite {
            return;
        }

        let (sin_track, cos_track) = fix.track.sin_cos();
        let ground_speed = fix.ground_speed.as_meters_per_second();
        let east = ground_speed * sin_track;
        let north = ground_speed * cos_track;

        // The estimate ages once per fix, whatever that fix can measure,
        // so it grows uncertain at the same rate either way.
        if let Some(interval) = self.interval_since_fix(time) {
            self.wind.predict(interval);
        }
        // One wind state takes both kinds of measurement, so it stays
        // continuous when an airspeed sensor appears or goes away.
        let air_speed = self.measured_air_speed.filter(|speed| speed.fresh_at(time));
        let fit = self.circling.update(time.as_secs_f64(), east, north);
        match air_speed {
            // An airspeed knows the radius of the circle, so it places
            // the wind better than a fit that has to find the radius too.
            Some(air_speed) => self.wind.update(
                east,
                north,
                Speed::from_meters_per_second(air_speed.value),
                fix.position_accuracy,
            ),
            None => {
                if let Some(fit) = fit {
                    self.wind
                        .update_vector(fit.east, fit.north, circling::MEASUREMENT_VARIANCE);
                }
            }
        }
        self.ground = Some(Timed {
            time,
            value: (east, north),
        });
        self.position = Some(fix.position);
    }

    fn interval_since_fix(&self, now: Duration) -> Option<f64> {
        let ground = self.ground?;
        now.checked_sub(ground.time)
            .filter(|interval| !interval.is_zero())
            .map(|interval| interval.as_secs_f64())
    }

    /// Takes a true airspeed from an instrument. It expires after five
    /// seconds, so the compensation stops when the instrument goes away.
    pub fn air_speed(&mut self, time: Duration, speed: Speed) {
        let speed = speed.as_meters_per_second();
        if !speed.is_finite() {
            return;
        }
        self.measured_air_speed = Some(Timed { time, value: speed });
    }

    /// The airspeed to work from: measured where an instrument reports
    /// one, otherwise what is left of the ground velocity once the wind
    /// is taken out.
    fn air_speed_at(&self, now: Duration) -> Option<f64> {
        if let Some(speed) = self.measured_air_speed.filter(|s| s.fresh_at(now)) {
            return Some(speed.value);
        }
        let ground = self.ground.filter(|ground| ground.fresh_at(now))?;
        let (east, north) = self.wind.vector()?;
        Some((ground.value.0 - east).hypot(ground.value.1 - north))
    }

    /// The air-relative track, which is the ground velocity with the wind
    /// taken out.
    fn heading(&self, now: Duration) -> Option<Angle> {
        let ground = self.ground.filter(|ground| ground.fresh_at(now))?;
        let (wind_east, wind_north) = self.wind.vector()?;
        let east = ground.value.0 - wind_east;
        let north = ground.value.1 - wind_north;
        (east.hypot(north) >= MIN_AIR_SPEED)
            .then(|| Angle::from_radians(east.atan2(north)).normalized())
    }

    /// Differentiates both the total energy height and the height alone.
    ///
    /// The airspeed term enters and leaves whole. Rebasing the previous
    /// height by it keeps the reading on the climb across the change,
    /// where restarting the filter would lose it: 57 m of energy at
    /// 120 km/h would otherwise read as 57 m of sink.
    fn advance_vertical_speed(&mut self, time: Duration, height: f64, barometric: bool) {
        let air_speed = self.air_speed_at(time);
        let energy = air_speed.map_or(0., |speed| speed * speed / (2. * GRAVITY));

        // Establishing the GNSS offset moves both heights by it.
        let fusion = self.height.take_step();

        // Its size at the previous sample is known where the term was
        // already there, and the current one stands in for it where it
        // was not.
        let compensation = match (self.previous_energy, air_speed.is_some()) {
            (None, true) => energy,
            (Some(previous), false) => -previous,
            _ => 0.,
        };
        self.previous_energy = air_speed.map(|_| energy);

        self.total_energy.advance(
            time,
            height + energy,
            fusion + compensation,
            barometric,
            self.vertical_speed_time_constant(),
        );
        self.uncompensated.advance(
            time,
            height,
            fusion,
            barometric,
            self.vertical_speed_time_constant(),
        );
    }

    /// The current estimate, or `None` until two altitudes have arrived
    /// close enough together to be differentiated.
    pub fn state(&self) -> Option<AirState> {
        // The uncompensated chain needs heights alone, so it is the one
        // that is always available.
        let rate_of_climb = self.uncompensated.value?;
        let vertical_speed = self.total_energy.value.unwrap_or(rate_of_climb);
        let now = self.uncompensated.previous?.time;
        let air_speed = self.air_speed_at(now);
        Some(AirState {
            vertical_speed: Speed::from_meters_per_second(vertical_speed),
            rate_of_climb: Speed::from_meters_per_second(rate_of_climb),
            air_speed: air_speed.map(Speed::from_meters_per_second),
            heading: self.heading(now),
            altitude: self.msl_altitude(),
            wind: self.wind.vector().map(|(east, north)| Wind {
                direction: Angle::from_radians((-east).atan2(-north)).normalized(),
                speed: Speed::from_meters_per_second(east.hypot(north)),
                uncertainty: Speed::from_meters_per_second(self.wind.uncertainty()),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_ge, assert_le, assert_lt, assert_none, assert_some};
    use updraft_units::Length;

    const POSITION: LatLon = LatLon::from_degrees(50.8, 6.2);

    fn fix(track: f64, ground_speed: f64) -> Fix {
        Fix {
            position: POSITION,
            track: Angle::from_degrees(track),
            ground_speed: Speed::from_kilometers_per_hour(ground_speed),
            position_accuracy: Length::from_meters(15.),
        }
    }

    fn meters(value: f64) -> PressureAltitude {
        PressureAltitude::new(Length::from_meters(value))
    }

    /// Climbs at `rate` m/s for `seconds` seconds, one sample each.
    fn climb(rate: f64, seconds: u64) -> AirStateEstimator {
        let mut estimator = AirStateEstimator::new();
        for second in 0..seconds {
            estimator.pressure_altitude(
                Duration::from_secs(second),
                meters(1000. + rate * second as f64),
            );
        }
        estimator
    }

    #[test]
    fn the_first_altitude_has_nothing_to_differentiate() {
        let mut estimator = AirStateEstimator::new();
        estimator.pressure_altitude(Duration::ZERO, meters(1000.));

        assert_none!(estimator.state());
    }

    #[test]
    fn a_steady_climb_reads_its_own_rate() {
        let state = assert_some!(climb(2., 60).state());

        assert_abs_diff_eq!(
            state.rate_of_climb,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn a_long_gap_restarts_the_vertical_speed() {
        let mut estimator = climb(2., 60);
        // Nothing can be differentiated across a gap this long, and the
        // height either side of it is unrelated.
        estimator.pressure_altitude(Duration::from_secs(120), meters(1120.));

        assert_none!(estimator.state());
    }

    #[test]
    fn a_repeated_timestamp_does_not_blank_the_reading() {
        let mut estimator = AirStateEstimator::new();
        for second in 0..60 {
            // Two sources of a pressure altitude reach the estimator in
            // one batch, so both carry the time the bytes arrived.
            let time = Duration::from_secs(second);
            let altitude = meters(1000. + 2. * second as f64);
            estimator.pressure_altitude(time, altitude);
            estimator.pressure_altitude(time, altitude);
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            state.rate_of_climb,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn a_source_timestamped_at_half_the_rate_still_reads_the_climb() {
        let mut estimator = AirStateEstimator::new();
        for half in 0..120 {
            // A 2 Hz source whose timestamps resolve to whole seconds:
            // every second arrives twice, and the height moves between
            // the two.
            estimator.pressure_altitude(Duration::from_secs(half / 2), meters(1000. + half as f64));
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            state.rate_of_climb,
            Speed::from_meters_per_second(2.),
            epsilon = 0.05
        );
    }

    /// Flies `seconds` seconds at a steady height, decelerating from
    /// 120 km/h by 1 km/h each second and trading the airspeed for
    /// height. `compensated` reports the airspeed to the estimator.
    fn pull_up(compensated: bool) -> AirStateEstimator {
        let mut estimator = AirStateEstimator::new();
        for second in 0..60 {
            let time = Duration::from_secs(second);
            let speed = (120. - second as f64) / 3.6;
            // Every metre per second of airspeed given up buys `v/g` of
            // height, so the total energy height does not move.
            let altitude = 1000. + (33.333 * 33.333 - speed * speed) / (2. * GRAVITY);
            if compensated {
                estimator.air_speed(time, Speed::from_meters_per_second(speed));
            }
            estimator.pressure_altitude(time, meters(altitude));
        }
        estimator
    }

    #[test]
    fn a_pull_up_trades_airspeed_for_height_without_a_climb() {
        let state = assert_some!(pull_up(true).state());

        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
        // The glider does gain height, and the uncompensated reading
        // reports it. Only the total-energy reading takes it out.
        assert!(
            state.rate_of_climb.as_meters_per_second() > 0.5,
            "rate of climb read {:?}",
            state.rate_of_climb
        );
    }

    #[test]
    fn without_an_airspeed_a_pull_up_reads_as_a_climb() {
        let state = assert_some!(pull_up(false).state());

        // Nothing separates height bought with airspeed from height
        // bought with lift, so both readings agree.
        assert_eq!(state.vertical_speed, state.rate_of_climb);
        assert!(state.vertical_speed.as_meters_per_second() > 0.5);
    }

    #[test]
    fn losing_the_airspeed_sensor_does_not_show_as_a_climb() {
        let mut estimator = AirStateEstimator::new();
        for second in 0..60 {
            let time = Duration::from_secs(second);
            estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
            estimator.pressure_altitude(time, meters(1000.));
        }

        // The energy term is 57 m at 120 km/h, and dropping it must not
        // read as 57 m of sink. The airspeed expires five seconds after
        // the last one arrived.
        for second in 60..70 {
            estimator.pressure_altitude(Duration::from_secs(second), meters(1000.));
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn an_airspeed_that_is_not_a_number_is_ignored() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut estimator = AirStateEstimator::new();
            for second in 0..60 {
                let time = Duration::from_secs(second);
                if second == 30 {
                    estimator.air_speed(time, Speed::from_meters_per_second(bad));
                }
                estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
                estimator.pressure_altitude(time, meters(1000. + 2. * second as f64));
            }

            let state = assert_some!(estimator.state(), "{bad}");
            assert_abs_diff_eq!(
                state.vertical_speed,
                Speed::from_meters_per_second(2.),
                epsilon = 0.01
            );
        }
    }

    /// Feeds `seconds` of pressure altitude at 25 Hz, climbing at 2 m/s
    /// with noise of standard deviation `sigma`, quantised to `step`.
    fn fast_barometer(sigma: f64, step: f64, seconds: u64) -> AirStateEstimator {
        let uniform = |seed: f64| {
            let x = (seed * 12.9898).sin() * 43758.5453;
            x - x.floor() - 0.5
        };
        let mut estimator = AirStateEstimator::new();
        for tick in 0..seconds * 25 {
            let time = Duration::from_secs_f64(tick as f64 / 25.);
            let i = tick as f64;
            let noise = 2. * sigma * (uniform(i) + uniform(i + 977.) + uniform(i + 1949.));
            let altitude = 1000. + 2. * (i / 25.) + noise;
            estimator.pressure_altitude(time, meters((altitude / step).round() * step));
        }
        estimator
    }

    #[test]
    fn a_quiet_fast_barometer_shortens_the_reading() {
        // 0.02 m of noise on a 25 Hz sensor that resolves it.
        let estimator = fast_barometer(0.02, 0.002, 30);

        assert_lt!(estimator.vertical_speed_time_constant(), 0.5);
        // The clamp holds it above the shortest reading that is still
        // worth smoothing at all.
        assert_ge!(estimator.vertical_speed_time_constant(), MIN_TIME_CONSTANT);
    }

    #[test]
    fn a_noisy_fast_barometer_lengthens_the_reading_to_its_limit() {
        // Ten times the noise the default was fitted at. The power law
        // would ask for 8.9 s, which the clamp holds at three.
        let estimator = fast_barometer(6., 0.002, 30);

        assert_le!(estimator.vertical_speed_time_constant(), MAX_TIME_CONSTANT);
        assert_ge!(estimator.vertical_speed_time_constant(), MAX_TIME_CONSTANT);
    }

    #[test]
    fn a_barometer_quieter_than_its_own_step_is_not_trusted() {
        // The same rate, but the reading is quantised far above its
        // noise, so the measurement cannot tell quiet from slow.
        let estimator = fast_barometer(0.02, 0.5, 30);

        assert_ge!(
            estimator.vertical_speed_time_constant(),
            DEFAULT_VERTICAL_SPEED_TIME_CONSTANT
        );
    }

    #[test]
    fn a_caller_setting_overrides_the_measurement() {
        let estimator = fast_barometer(0.02, 0.002, 30)
            .with_vertical_speed_time_constant(Duration::from_secs(2));

        assert_eq!(estimator.vertical_speed_time_constant(), 2.);
    }

    #[test]
    fn a_shorter_time_constant_reaches_the_climb_rate_sooner() {
        /// The two stages lag by about twice the time constant, so a
        /// quarter-second one is settled where a two-second one is not.
        fn climb_after(seconds: u64, time_constant: Duration) -> Speed {
            let mut estimator =
                AirStateEstimator::new().with_vertical_speed_time_constant(time_constant);
            for tenth in 0..seconds * 10 {
                let time = Duration::from_millis(tenth * 100);
                estimator.pressure_altitude(time, meters(0.2 * tenth as f64));
            }
            estimator.state().map_or(Speed::ZERO, |s| s.vertical_speed)
        }

        assert_abs_diff_eq!(
            climb_after(2, Duration::from_millis(250)),
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
        assert_lt!(
            climb_after(2, Duration::from_secs(2)),
            Speed::from_meters_per_second(1.)
        );
    }

    #[test]
    fn the_airspeed_sensor_is_reported_as_it_arrives() {
        let mut estimator = AirStateEstimator::new();
        for step in 0..10u64 {
            let time = Duration::from_secs(step);
            estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
            estimator.fix(time, &fix(90., 150.));
            estimator.pressure_altitude(time, meters(1000.));
        }

        // The ground speed is 150 km/h, so this can only be the sensor.
        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            assert_some!(state.air_speed),
            Speed::from_kilometers_per_hour(120.),
            epsilon = 1e-9
        );
    }

    #[test]
    fn circling_recovers_the_airspeed_and_heading_without_a_sensor() {
        // The last fix of the circle points 342°, at 108 km/h.
        let state = assert_some!(circle_without_a_sensor(60).state());

        assert_abs_diff_eq!(
            assert_some!(state.air_speed),
            Speed::from_kilometers_per_hour(108.),
            epsilon = 0.3
        );
        assert_abs_diff_eq!(
            assert_some!(state.heading),
            Angle::from_degrees(342.),
            epsilon = 0.5
        );
    }

    #[test]
    fn the_heading_waits_for_the_wind() {
        // Without the wind the heading would only repeat the track over
        // ground that the caller supplied.
        assert_none!(assert_some!(circle_without_a_sensor(10).state()).heading);
        assert_some!(assert_some!(circle_without_a_sensor(60).state()).heading);
    }

    #[test]
    fn the_altitude_needs_a_gnss_altitude_for_its_reference() {
        let mut estimator = AirStateEstimator::new();
        for step in 0..10u64 {
            let time = Duration::from_secs(step);
            estimator.fix(time, &fix(90., 120.));
            estimator.pressure_altitude(time, meters(1000.));
        }

        // A pressure altitude alone says nothing about sea level.
        assert_none!(assert_some!(estimator.state()).altitude);
    }

    #[test]
    fn the_altitude_follows_the_gnss_altitude_through_the_geoid() {
        let mut estimator = AirStateEstimator::new();
        for step in 0..30u64 {
            let time = Duration::from_secs(step);
            estimator.fix(time, &fix(90., 120.));
            estimator.gnss_altitude(time, EllipsoidAltitude::new(Length::from_meters(1046.)));
            estimator.pressure_altitude(time, meters(1000.));
        }

        let expected = updraft_egm96::ellipsoidal_to_msl(
            POSITION,
            EllipsoidAltitude::new(Length::from_meters(1046.)),
        );
        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(assert_some!(state.altitude), expected, epsilon = 0.01);
    }

    #[test]
    fn losing_the_barometer_keeps_the_altitude_it_was_reporting() {
        let mut estimator = AirStateEstimator::new();
        let ellipsoidal = |value: f64| EllipsoidAltitude::new(Length::from_meters(value));
        let mut fused = None;

        for step in 0..120u64 {
            let time = Duration::from_secs(step);
            estimator.fix(time, &fix(90., 120.));
            // The two altitudes are 200 m apart. The barometer stops
            // after a minute; the receiver keeps reporting.
            if step < 60 {
                estimator.pressure_altitude(time, meters(1000.));
            }
            estimator.gnss_altitude(time, ellipsoidal(1200.));
            if step == 59 {
                fused = assert_some!(estimator.state()).altitude;
            }
        }

        // The glider has not moved, so the altitude has not either. The
        // offset belongs between the two altitudes, and the GNSS one is
        // already on its far side: adding it again would move the
        // reading by the whole datum difference.
        let before = assert_some!(fused);
        let after = assert_some!(assert_some!(estimator.state()).altitude);
        assert_abs_diff_eq!(after, before, epsilon = 1.);
    }

    #[test]
    fn an_altitude_that_is_not_a_number_is_ignored() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut estimator = climb(2., 30);
            estimator.pressure_altitude(Duration::from_secs(30), meters(bad));
            for second in 31..60 {
                estimator.pressure_altitude(
                    Duration::from_secs(second),
                    meters(1000. + 2. * second as f64),
                );
            }

            // A NaN reaches every stage of the filter and none of them
            // can heal, so it has to be rejected where it arrives.
            let state = assert_some!(estimator.state(), "{bad}");
            assert_abs_diff_eq!(
                state.rate_of_climb,
                Speed::from_meters_per_second(2.),
                epsilon = 0.01
            );
        }
    }

    #[test]
    fn a_faster_barometer_gives_the_same_climb_rate() {
        // Ten samples per second instead of one. The smoothing weight
        // follows the interval, so the reading is the same climb.
        let mut estimator = AirStateEstimator::new();
        for tenth in 0..600u64 {
            let time = Duration::from_millis(tenth * 100);
            estimator.pressure_altitude(time, meters(1000. + 0.2 * tenth as f64));
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            state.rate_of_climb,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn gaining_the_airspeed_sensor_does_not_show_as_a_climb() {
        let mut estimator = AirStateEstimator::new();
        for second in 0..60u64 {
            estimator.pressure_altitude(Duration::from_secs(second), meters(1000.));
        }

        // The energy term is 57 m at 120 km/h. Adding it must not read
        // as 57 m of climb.
        for second in 60..70u64 {
            let time = Duration::from_secs(second);
            estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
            estimator.pressure_altitude(time, meters(1000.));
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn the_rate_of_climb_survives_losing_the_airspeed_sensor() {
        let mut estimator = AirStateEstimator::new();
        for second in 0..60u64 {
            let time = Duration::from_secs(second);
            estimator.air_speed(time, Speed::from_kilometers_per_hour(120.));
            estimator.pressure_altitude(time, meters(1000. + 2. * second as f64));
        }

        // Dropping the energy term takes 57 m out of the total energy
        // height. Rebasing the previous height by the same amount keeps
        // both readings on the 2 m/s climb across the change.
        let mut lowest_rate = f64::INFINITY;
        let mut lowest_vertical = f64::INFINITY;
        for second in 60..70u64 {
            let time = Duration::from_secs(second);
            estimator.pressure_altitude(time, meters(1000. + 2. * second as f64));
            let state = assert_some!(estimator.state());
            lowest_rate = lowest_rate.min(state.rate_of_climb.as_meters_per_second());
            lowest_vertical = lowest_vertical.min(state.vertical_speed.as_meters_per_second());
        }

        assert!(lowest_rate >= 1.99, "rate of climb dipped to {lowest_rate}");
        assert!(
            lowest_vertical >= 1.99,
            "vertical speed dipped to {lowest_vertical}"
        );
    }

    #[test]
    fn a_receiver_without_a_barometer_still_gives_a_vertical_speed() {
        let mut estimator = AirStateEstimator::new();
        for second in 0..60u64 {
            estimator.gnss_altitude(
                Duration::from_secs(second),
                EllipsoidAltitude::new(Length::from_meters(1200. + 2. * second as f64)),
            );
        }

        let state = assert_some!(estimator.state());
        assert_abs_diff_eq!(
            state.rate_of_climb,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn establishing_the_gnss_offset_does_not_show_as_a_climb() {
        let mut estimator = AirStateEstimator::new();
        // A minute on the barometer alone, then the receiver joins with
        // an altitude 200 m away from it.
        for second in 0..60u64 {
            estimator.pressure_altitude(Duration::from_secs(second), meters(1000.));
        }

        let mut worst = 0f64;
        for second in 60..90u64 {
            let time = Duration::from_secs(second);
            estimator.gnss_altitude(time, EllipsoidAltitude::new(Length::from_meters(1200.)));
            estimator.pressure_altitude(time, meters(1000.));
            let state = assert_some!(estimator.state());
            worst = worst.max(state.rate_of_climb.as_meters_per_second().abs());
        }

        // The height moves by the whole 200 m offset when it is first
        // established. That is a change of reference, not a climb.
        assert!(worst < 0.01, "reading reached {worst} m/s");
    }

    #[test]
    fn an_impossible_time_constant_is_ignored() {
        let estimator = AirStateEstimator::new()
            .with_vertical_speed_time_constant(Duration::from_millis(250))
            .with_vertical_speed_time_constant(Duration::ZERO);

        assert_eq!(estimator.vertical_speed_time_constant(), 0.25);
    }

    /// Circles at 108 km/h through a 10 m/s wind from the north-east,
    /// with no airspeed sensor, so only the shape of the circle can
    /// place the wind.
    fn circle_without_a_sensor(seconds: u64) -> AirStateEstimator {
        const TURN_SECONDS: f64 = 20.;
        let air_speed = Speed::from_kilometers_per_hour(108.).as_meters_per_second();
        let mut estimator = AirStateEstimator::new();
        for step in 0..seconds {
            let heading = std::f64::consts::TAU * step as f64 / TURN_SECONDS;
            let east = -6. + air_speed * heading.sin();
            let north = -8. + air_speed * heading.cos();
            let time = Duration::from_secs(step);
            estimator.fix(
                time,
                &Fix {
                    position: LatLon::from_degrees(50.8, 6.2),
                    track: Angle::from_radians(east.atan2(north)),
                    ground_speed: Speed::from_meters_per_second(east.hypot(north)),
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
        assert_abs_diff_eq!(wind.direction, Angle::from_degrees(36.87), epsilon = 0.5);
    }

    #[test]
    fn the_wind_survives_losing_the_airspeed_sensor() {
        const TURN_SECONDS: f64 = 20.;
        let air_speed = Speed::from_kilometers_per_hour(108.).as_meters_per_second();
        let mut estimator = AirStateEstimator::new();
        let mut worst = 0f64;

        // Circles through the same wind, with the sensor for the first
        // half of the time and without it for the second. One filter
        // takes both kinds of measurement, so nothing restarts.
        for step in 0..120u64 {
            let heading = std::f64::consts::TAU * step as f64 / TURN_SECONDS;
            let east = -6. + air_speed * heading.sin();
            let north = -8. + air_speed * heading.cos();
            let time = Duration::from_secs(step);
            if step < 60 {
                estimator.air_speed(time, Speed::from_meters_per_second(air_speed));
            }
            estimator.fix(
                time,
                &Fix {
                    position: LatLon::from_degrees(50.8, 6.2),
                    track: Angle::from_radians(east.atan2(north)),
                    ground_speed: Speed::from_meters_per_second(east.hypot(north)),
                    position_accuracy: Length::from_meters(15.),
                },
            );
            estimator.pressure_altitude(time, meters(1000.));
            if step > 60 {
                let wind = assert_some!(assert_some!(estimator.state()).wind);
                worst = worst.max((wind.speed.as_meters_per_second() - 10.).abs());
            }
        }

        assert!(worst < 0.5, "wind speed drifted {worst} m/s");
    }
}
