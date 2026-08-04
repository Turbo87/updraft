use crate::wind::{Wind, WindFilter};
use std::time::Duration;
use updraft_polar::GlidePolar;
use updraft_units::{Angle, Length, PressureAltitude, Speed};

/// Standard gravity, in m/s².
const GRAVITY: f64 = 9.80665;

/// Time constant of each of the two vertical-speed smoothing stages, in
/// seconds. Fitted against the recorded vario of an LXNAV LX9070 in
/// `testdata/weglide_1141558.igc`.
const VERTICAL_SPEED_TIME_CONSTANT: f64 = 2.;

/// Time constant of the turn-rate smoothing, in seconds. It only has to
/// suppress the fix-to-fix track noise: the turn rate itself changes
/// slowly, and the sink rate reacts to it weakly.
const TURN_RATE_TIME_CONSTANT: f64 = 3.;

/// The longest gap that two samples can still be differentiated across.
/// A larger gap restarts the estimate.
const MAX_SAMPLE_INTERVAL: Duration = Duration::from_secs(30);

/// Airspeed below which the air-relative track is meaningless, in m/s.
const MIN_AIR_SPEED: f64 = 1.;

/// Load factor the sink rate is capped at. A steeper turn than 70° of
/// bank is turbulence or track noise, not circling.
const MAX_LOAD_FACTOR: f64 = 3.;

/// One set of measurements, as a flight recorder logs them.
///
/// The position itself is not needed: track and ground speed already
/// carry the ground velocity, and pressure altitude replaces the GNSS
/// altitude, which is too noisy to differentiate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// Track over ground, clockwise from true north.
    pub track: Angle,
    /// Speed over ground.
    pub ground_speed: Speed,
    /// Barometric altitude against the 1013.25 hPa datum.
    pub pressure_altitude: PressureAltitude,
    /// True airspeed.
    pub true_air_speed: Speed,
    /// Horizontal accuracy the GNSS receiver reports for this fix.
    pub position_accuracy: Length,
}

/// What the glider and the air around it are doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirState {
    /// Total-energy vertical speed: how fast the glider gains height,
    /// with the height it trades for airspeed taken out. Positive means
    /// climbing.
    pub vertical_speed: Speed,
    /// Vertical speed of the air mass: the total-energy vertical speed
    /// with the glider's own sink rate added back. Positive means rising
    /// air.
    pub netto: Speed,
    /// Horizontal movement of the air mass, or `None` while the wind
    /// estimate has not converged yet.
    pub wind: Option<Wind>,
}

/// Derives vertical speed, netto and wind from recorded flight data.
///
/// Feed every sample to [`update`](Self::update) in recording order. The
/// estimator keeps the state between samples, so it works the same on a
/// live stream and on a replayed file. Sample intervals may vary; the
/// filters use the interval that each sample reports.
///
/// The three outputs build on each other:
///
/// 1. The **total energy height** `h + v²/2g` removes the height that
///    the glider trades against airspeed when it pushes or pulls. Its
///    derivative is the total-energy **vertical speed**. Two smoothing
///    stages suppress the metre-resolution steps of the logged pressure
///    altitude.
/// 2. The **wind** comes from the difference between the ground velocity
///    and the true airspeed (see [`WindFilter`]).
/// 3. Subtracting the wind from the ground velocity gives the air-relative
///    track. Its rate of change is the turn rate, which fixes the bank
///    angle and therefore the load factor. The load factor and the air
///    density at the current altitude turn the glide polar into the
///    current sink rate, and the vertical speed plus that sink rate is
///    the **netto**.
#[derive(Clone, Copy, Debug)]
pub struct AirStateEstimator {
    polar: GlidePolar,
    wind: WindFilter,
    previous: Option<Previous>,
    /// Air-relative track of the previous sample, absent while too slow.
    previous_heading: Option<f64>,
    first_stage: f64,
    vertical_speed: f64,
    turn_rate: f64,
}

#[derive(Clone, Copy, Debug)]
struct Previous {
    time: Duration,
    total_energy_height: f64,
}

impl AirStateEstimator {
    /// Creates an estimator for a glider with the given polar. The polar
    /// must already carry the flight's mass and bugs settings, because
    /// they scale the sink rate that the netto builds on.
    pub fn new(polar: GlidePolar) -> Self {
        Self {
            polar,
            wind: WindFilter::default(),
            previous: None,
            previous_heading: None,
            first_stage: 0.,
            vertical_speed: 0.,
            turn_rate: 0.,
        }
    }

    /// Folds one sample into the estimate. `time` is the sample's own
    /// time, not the time it arrived.
    ///
    /// Returns `None` for the first sample and after a gap longer than
    /// 30 seconds, because vertical speed needs two samples to be
    /// differentiated.
    pub fn update(&mut self, time: Duration, sample: &Sample) -> Option<AirState> {
        let interval = self
            .previous
            .and_then(|previous| time.checked_sub(previous.time))
            .filter(|interval| !interval.is_zero() && *interval <= MAX_SAMPLE_INTERVAL)
            .map(|interval| interval.as_secs_f64());

        let (sin_track, cos_track) = sample.track.sin_cos();
        let ground_speed = sample.ground_speed.as_meters_per_second();
        let ground_east = ground_speed * sin_track;
        let ground_north = ground_speed * cos_track;

        self.wind.update(
            interval,
            ground_east,
            ground_north,
            sample.true_air_speed,
            sample.position_accuracy,
        );
        self.update_turn_rate(interval, ground_east, ground_north);

        let air_speed = sample.true_air_speed.as_meters_per_second();
        let altitude = sample.pressure_altitude.into_inner().as_meters();
        let total_energy_height = altitude + air_speed * air_speed / (2. * GRAVITY);
        let previous = self.previous.replace(Previous {
            time,
            total_energy_height,
        });

        let Some((interval, previous)) = interval.zip(previous) else {
            self.first_stage = 0.;
            self.vertical_speed = 0.;
            return None;
        };

        let raw = (total_energy_height - previous.total_energy_height) / interval;
        let weight = smoothing_weight(interval, VERTICAL_SPEED_TIME_CONSTANT);
        self.first_stage += weight * (raw - self.first_stage);
        self.vertical_speed += weight * (self.first_stage - self.vertical_speed);

        let sink_rate = self.sink_rate(altitude, air_speed);
        Some(AirState {
            vertical_speed: Speed::from_meters_per_second(self.vertical_speed),
            netto: Speed::from_meters_per_second(self.vertical_speed + sink_rate),
            wind: self.wind.wind(),
        })
    }

    /// Tracks the rate of change of the air-relative track, which is what
    /// a turn coordinator measures.
    fn update_turn_rate(&mut self, interval: Option<f64>, ground_east: f64, ground_north: f64) {
        let air_east = ground_east - self.wind.east();
        let air_north = ground_north - self.wind.north();
        if air_east.hypot(air_north) < MIN_AIR_SPEED {
            self.previous_heading = None;
            return;
        }

        let heading = air_east.atan2(air_north);
        if let Some((interval, previous)) = interval.zip(self.previous_heading) {
            let change = Angle::from_radians(heading - previous)
                .normalized_signed()
                .as_radians();
            let weight = smoothing_weight(interval, TURN_RATE_TIME_CONSTANT);
            self.turn_rate += weight * (change / interval - self.turn_rate);
        }
        self.previous_heading = Some(heading);
    }

    /// The still-air sink rate (a positive number) at the current
    /// airspeed, load factor and air density, in m/s.
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

/// Weight of a new value in an exponential filter with the given time
/// constant, for a sample interval that is not fixed.
fn smoothing_weight(interval: f64, time_constant: f64) -> f64 {
    1. - (-interval / time_constant).exp()
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
    use updraft_polar::POLAR_STORE;

    fn polar() -> GlidePolar {
        POLAR_STORE
            .iter()
            .find(|entry| entry.name == "JS-3-18m")
            .expect("the built-in store has a JS-3-18m polar")
            .glide_polar()
    }

    fn sample(track: f64, ground_speed: f64, altitude: f64, air_speed: f64) -> Sample {
        Sample {
            track: Angle::from_degrees(track),
            ground_speed: Speed::from_kilometers_per_hour(ground_speed),
            pressure_altitude: PressureAltitude::new(Length::from_meters(altitude)),
            true_air_speed: Speed::from_kilometers_per_hour(air_speed),
            position_accuracy: Length::from_meters(15.),
        }
    }

    #[test]
    fn the_first_sample_has_nothing_to_differentiate() {
        let mut estimator = AirStateEstimator::new(polar());

        assert_none!(estimator.update(Duration::ZERO, &sample(90., 120., 1000., 120.)));
        assert_some!(estimator.update(Duration::from_secs(1), &sample(90., 120., 1001., 120.)));
    }

    #[test]
    fn a_long_gap_restarts_the_vertical_speed() {
        let mut estimator = AirStateEstimator::new(polar());
        for second in 0..60 {
            estimator.update(
                Duration::from_secs(second),
                &sample(90., 120., 1000. + second as f64, 120.),
            );
        }

        assert_none!(estimator.update(Duration::from_secs(120), &sample(90., 120., 1120., 120.)));
        let state = assert_some!(
            estimator.update(Duration::from_secs(121), &sample(90., 120., 1120., 120.))
        );
        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn a_steady_climb_converges_on_its_rate() {
        let mut estimator = AirStateEstimator::new(polar());
        let mut state = None;
        for second in 0..60 {
            state = estimator.update(
                Duration::from_secs(second),
                &sample(90., 120., 1000. + 2. * second as f64, 120.),
            );
        }

        let state = assert_some!(state);
        assert_abs_diff_eq!(
            state.vertical_speed,
            Speed::from_meters_per_second(2.),
            epsilon = 0.01
        );
    }

    #[test]
    fn a_pull_up_trades_airspeed_for_height_without_a_climb() {
        let mut estimator = AirStateEstimator::new(polar());
        let mut state = None;
        // Decelerating from 160 to 100 km/h gains the matching height.
        for second in 0..60 {
            let air_speed = 160. - second as f64;
            let air_speed = air_speed.max(100.);
            let speed = Speed::from_kilometers_per_hour(air_speed).as_meters_per_second();
            let altitude = 1000. + (44.44 * 44.44 - speed * speed) / (2. * GRAVITY);
            state = estimator.update(
                Duration::from_secs(second),
                &sample(90., air_speed, altitude, air_speed),
            );
        }

        let state = assert_some!(state);
        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
    }

    #[test]
    fn netto_adds_the_sink_rate_of_the_glider() {
        let mut estimator = AirStateEstimator::new(polar());
        let mut state = None;
        for second in 0..60 {
            state = estimator.update(Duration::from_secs(second), &sample(90., 120., 1000., 120.));
        }

        // Level flight at 120 km/h and 1000 m sinks at 0.60 m/s, so air
        // that holds the glider level must rise at the same rate.
        let state = assert_some!(state);
        assert_abs_diff_eq!(state.vertical_speed, Speed::ZERO, epsilon = 0.01);
        assert_abs_diff_eq!(
            state.netto,
            Speed::from_meters_per_second(0.603),
            epsilon = 0.005
        );
    }

    #[test]
    fn circling_raises_the_sink_rate_the_netto_corrects_for() {
        let mut estimator = AirStateEstimator::new(polar());
        let mut state = None;
        for second in 0..120 {
            let track = 360. * second as f64 / 20.;
            state = estimator.update(
                Duration::from_secs(second),
                &sample(track, 108., 1000., 108.),
            );
        }

        // 108 km/h in a 20 s circle is 44° of bank, which raises the sink
        // rate from the 0.571 m/s of the same speed with level wings.
        let state = assert_some!(state);
        assert_abs_diff_eq!(
            state.netto - state.vertical_speed,
            Speed::from_meters_per_second(0.645),
            epsilon = 0.005
        );
    }

    #[test]
    fn the_isa_model_matches_the_published_density_ratios() {
        assert_abs_diff_eq!(isa_density_ratio(0.), 1., epsilon = 1e-9);
        assert_abs_diff_eq!(isa_density_ratio(1000.), 0.9075, epsilon = 5e-4);
        assert_abs_diff_eq!(isa_density_ratio(3000.), 0.7422, epsilon = 5e-4);
    }
}
