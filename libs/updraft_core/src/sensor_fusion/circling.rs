use std::collections::VecDeque;
use std::f64::consts::TAU;
use std::time::Duration;
use updraft_units::{Angle, Speed};

const FULL_TURN: Angle = Angle::from_radians(TAU);

/// Shortest gap between buffered samples, in seconds. It keeps the buffer
/// spanning the same amount of *time* whatever rate the fixes arrive at.
const MIN_SPACING: Duration = Duration::from_millis(500);

/// Longest history the fit looks back over, in seconds. A circle that
/// takes longer than this is not a thermalling circle.
const MAX_WINDOW: Duration = Duration::from_secs(60);

/// Samples the buffer holds, enough for [`MAX_WINDOW`] at [`MIN_SPACING`].
const CAPACITY: usize = 128;

/// Fewest samples a fit needs.
const MIN_SAMPLES: usize = 8;

/// How far back the turn is checked for still being a turn, in seconds.
const TURNING_WINDOW: Duration = Duration::from_secs(5);

/// How far the track has to sweep over [`TURNING_WINDOW`] for the glider
/// to still count as circling, in radians. It admits a circle of up to
/// 40 seconds, which is slower than any thermalling turn.
const TURNING_ANGLE: Angle = Angle::from_radians(TAU / 8.);

/// Airspeeds a circling glider can plausibly fly, in m/s. A fitted radius
/// outside this is a turn that is not a circle.
const MIN_AIR_SPEED: Speed = Speed::from_meters_per_second(15.);
const MAX_AIR_SPEED: Speed = Speed::from_meters_per_second(70.);

/// Largest scatter around the fitted circle that is still accepted, in m/s.
const MAX_RESIDUAL: Speed = Speed::from_meters_per_second(3.);

/// Variance of one circle measurement, in `(m/s)²`.
///
/// It is a constant. Scaling it with the scatter around the fitted
/// circle was measured and rejected: it moved the wind of the recorded
/// flight from 2.13 to 2.17 m/s RMS. A fit that closes at all measures
/// its own circle well, and what limits the measurement is how much the
/// wind changed while the glider flew that circle.
///
/// A variance of 0.5 makes the filter average consecutive circles. It
/// reduced the wind RMS of the recorded flight without an airspeed
/// sensor from 2.04 to 1.93 m/s. Larger values did not improve accuracy
/// and slowed the response to wind changes. A value above 0.5 also keeps
/// the first circle from reaching the wind filter's reporting threshold.
pub const MEASUREMENT_VARIANCE: f64 = 0.5;

/// Measures the wind from the shape of a circle, without an airspeed
/// measurement.
///
/// Through a full circle at a steady airspeed, the ground velocity traces
/// a circle in velocity space. Its centre is the wind and its radius is
/// the airspeed. Fitting all three leaves the measurement weaker than the
/// airspeed-driven one, which knows the radius and only has to place the
/// centre, and it only produces a value while the glider circles.
///
/// This produces measurements. [`WindFilter`](super::wind::WindFilter)
/// holds the wind, so that the estimate stays continuous when an airspeed
/// sensor appears or goes away.
#[derive(Clone, Debug, Default)]
pub struct CirclingWind {
    samples: VecDeque<Sample>,
    /// Track of the newest sample, unwrapped so that a full turn shows as
    /// a change of 2π rather than wrapping around.
    track: Angle,
    /// Unwrapped track of the last successful wind measurement.
    last_measurement_track: Option<Angle>,
}

#[derive(Clone, Copy, Debug)]
struct Sample {
    time: Duration,
    east: Speed,
    north: Speed,
    track: Angle,
}

/// One wind measurement from a closed circle, in m/s towards east and
/// north. Its variance is [`MEASUREMENT_VARIANCE`].
#[derive(Clone, Copy, Debug)]
pub struct Fit {
    pub east: Speed,
    pub north: Speed,
    measurement_track: Angle,
}

impl CirclingWind {
    /// Folds one ground velocity in and returns a candidate whenever a
    /// complete circle closes.
    pub fn update(&mut self, time: Duration, east: Speed, north: Speed) -> Option<Fit> {
        let track = unwrapped_track(east, north, self.track)?;
        self.track = track;
        self.push(Sample {
            time,
            east,
            north,
            track,
        });

        let start = self.completed_turn()?;
        let measurement_track = self.samples.back()?.track;
        if self
            .last_measurement_track
            .is_some_and(|previous| (measurement_track - previous).abs() < FULL_TURN)
        {
            return None;
        }
        let points: Vec<_> = self
            .samples
            .iter()
            .skip(start)
            .map(|sample| {
                let east = sample.east.as_meters_per_second();
                let north = sample.north.as_meters_per_second();
                (east, north)
            })
            .collect();
        fit_circle(&points, measurement_track)
    }

    /// Records that the candidate was applied to the wind estimate.
    pub fn accept(&mut self, fit: Fit) {
        self.last_measurement_track = Some(fit.measurement_track);
    }

    fn push(&mut self, sample: Sample) {
        // Time running backwards means the buffer describes a different
        // flight, or a recording that crossed midnight without saying so.
        if self
            .samples
            .back()
            .is_some_and(|last| sample.time < last.time)
        {
            self.samples.clear();
            self.last_measurement_track = None;
        }
        let spaced = self
            .samples
            .back()
            .is_none_or(|last| sample.time - last.time >= MIN_SPACING);
        if !spaced {
            return;
        }
        self.samples.push_back(sample);
        while self.samples.len() > CAPACITY
            || self
                .samples
                .front()
                .is_some_and(|first| sample.time - first.time > MAX_WINDOW)
        {
            self.samples.pop_front();
        }
    }

    /// Index of the newest buffered sample that a full turn separates from
    /// the current one, so that the fit uses the shortest complete circle.
    ///
    /// The glider also has to still be turning. A buffer that merely
    /// *contains* a circle keeps fitting for a whole window after the
    /// pilot rolls out, over that circle plus a growing straight tail,
    /// and a thermal exit is normally flown with an acceleration to
    /// cruise: the tail then sits on a circle of a different radius and
    /// drags the fit off the wind it had.
    fn completed_turn(&self) -> Option<usize> {
        let latest = self.samples.back()?;
        if !self.is_turning() {
            return None;
        }
        (0..=self.samples.len().checked_sub(MIN_SAMPLES)?)
            .rev()
            .find(|&index| (latest.track - self.samples[index].track).abs() >= FULL_TURN)
    }

    /// Whether the track is still sweeping, over the newest few seconds.
    pub fn is_turning(&self) -> bool {
        let Some(latest) = self.samples.back() else {
            return false;
        };
        self.samples
            .iter()
            .rev()
            .find(|sample| latest.time - sample.time >= TURNING_WINDOW)
            .is_some_and(|earlier| (latest.track - earlier.track).abs() >= TURNING_ANGLE)
    }
}

/// The track the velocity points along, continued from `previous` instead
/// of wrapping. `None` while the glider is too slow for a track.
fn unwrapped_track(east: Speed, north: Speed, previous: Angle) -> Option<Angle> {
    let east = east.as_meters_per_second();
    let north = north.as_meters_per_second();
    let speed = Speed::from_meters_per_second(east.hypot(north));
    if speed < Speed::from_meters_per_second(1.) {
        return None;
    }
    let change = Angle::from_radians(east.atan2(north) - previous.as_radians()).normalized_signed();
    Some(previous + change)
}

/// Fits a circle through velocity-space points, by the algebraic
/// (Kåsa) method on points shifted to their own mean.
fn fit_circle(points: &[(f64, f64)], measurement_track: Angle) -> Option<Fit> {
    if points.len() < MIN_SAMPLES {
        return None;
    }
    let count = points.len() as f64;
    let mean_east = points.iter().map(|p| p.0).sum::<f64>() / count;
    let mean_north = points.iter().map(|p| p.1).sum::<f64>() / count;

    let (mut xx, mut xy, mut yy, mut xz, mut yz, mut z_sum) = (0., 0., 0., 0., 0., 0.);
    for &(east, north) in points {
        let x = east - mean_east;
        let y = north - mean_north;
        let z = x * x + y * y;
        xx += x * x;
        xy += x * y;
        yy += y * y;
        xz += x * z;
        yz += y * z;
        z_sum += z;
    }

    // Shifting to the mean zeroes the sums of x and y, which decouples the
    // centre from the radius and leaves a 2x2 system.
    let determinant = xx * yy - xy * xy;
    if determinant <= 0. {
        return None;
    }
    let east = (yy * xz - xy * yz) / (2. * determinant);
    let north = (xx * yz - xy * xz) / (2. * determinant);
    let air_speed = (z_sum / count + east * east + north * north).sqrt();
    let air_speed = Speed::from_meters_per_second(air_speed);
    if !(MIN_AIR_SPEED..=MAX_AIR_SPEED).contains(&air_speed) {
        return None;
    }

    let east = mean_east + east;
    let north = mean_north + north;
    let residual = (points
        .iter()
        .map(|&(e, n)| ((e - east).hypot(n - north) - air_speed.as_meters_per_second()).powi(2))
        .sum::<f64>()
        / count)
        .sqrt();
    (Speed::from_meters_per_second(residual) <= MAX_RESIDUAL).then_some(Fit {
        east: Speed::from_meters_per_second(east),
        north: Speed::from_meters_per_second(north),
        measurement_track,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some};

    fn speed(value: f64) -> Speed {
        Speed::from_meters_per_second(value)
    }

    fn update(wind: &mut CirclingWind, time: f64, east: f64, north: f64) -> Option<Fit> {
        wind.update(Duration::from_secs_f64(time), speed(east), speed(north))
    }

    /// Circles for `seconds` at `AIR_SPEED` through a wind of
    /// `(east, north)` m/s, one sample per second. Returns the last
    /// measurement the circles produced.
    fn circle(seconds: usize, east: f64, north: f64) -> Option<Fit> {
        const TURN_SECONDS: f64 = 20.;
        const AIR_SPEED: f64 = 30.;

        let mut wind = CirclingWind::default();
        let mut last = None;
        for second in 0..seconds {
            let heading = TAU * second as f64 / TURN_SECONDS;
            let east = east + AIR_SPEED * heading.sin();
            let north = north + AIR_SPEED * heading.cos();
            if let Some(fit) = update(&mut wind, second as f64, east, north) {
                wind.accept(fit);
                last = Some(fit);
            }
        }
        last
    }

    #[test]
    fn full_circle_produces_wind() {
        let fit = assert_some!(circle(30, -6., -8.));

        assert_abs_diff_eq!(fit.east, speed(-6.), epsilon = 0.2);
        assert_abs_diff_eq!(fit.north, speed(-8.), epsilon = 0.2);
    }

    #[test]
    fn minimum_samples_produce_wind() {
        let mut wind = CirclingWind::default();
        let mut fit = None;
        for sample in 0..MIN_SAMPLES {
            let heading = TAU * sample as f64 / (MIN_SAMPLES - 1) as f64;
            fit = update(
                &mut wind,
                sample as f64,
                -6. + 30. * heading.sin(),
                -8. + 30. * heading.cos(),
            );
        }

        let fit = assert_some!(fit);
        assert_abs_diff_eq!(fit.east, speed(-6.), epsilon = 0.2);
        assert_abs_diff_eq!(fit.north, speed(-8.), epsilon = 0.2);
    }

    #[test]
    fn partial_circle_produces_no_wind() {
        assert_none!(circle(15, -6., -8.));
    }

    #[test]
    fn straight_glide_produces_no_wind() {
        let mut wind = CirclingWind::default();
        for second in 0..120 {
            assert_none!(update(&mut wind, second as f64, 40., 3.));
        }
    }

    #[test]
    fn time_running_backwards_starts_a_new_buffer() {
        let mut wind = CirclingWind::default();
        for second in 0..30 {
            let heading = TAU * second as f64 / 20.;
            let east = 30. * heading.sin();
            let north = 30. * heading.cos();
            let _ = update(&mut wind, second as f64, east, north);
        }

        // A second circle timed before the first must not be fitted
        // together with it, so no complete turn is available again.
        for second in 0..15 {
            let heading = TAU * second as f64 / 20.;
            let east = 30. * heading.sin();
            let north = 30. * heading.cos();
            assert_none!(update(&mut wind, second as f64, east, north));
        }
    }

    #[test]
    fn fix_rate_does_not_change_wind() {
        const TURN_SECONDS: f64 = 20.;
        const AIR_SPEED: f64 = 30.;

        let mut wind = CirclingWind::default();
        let mut last = None;
        for tenth in 0..300 {
            let time = tenth as f64 / 10.;
            let heading = TAU * time / TURN_SECONDS;
            let east = -6. + AIR_SPEED * heading.sin();
            let north = -8. + AIR_SPEED * heading.cos();
            last = update(&mut wind, time, east, north).or(last);
        }

        let fit = assert_some!(last);
        assert_abs_diff_eq!(fit.east, speed(-6.), epsilon = 0.2);
        assert_abs_diff_eq!(fit.north, speed(-8.), epsilon = 0.2);
    }

    #[test]
    fn one_turn_produces_one_measurement() {
        const TURN_SECONDS: f64 = 20.;
        const AIR_SPEED: f64 = 30.;

        let mut wind = CirclingWind::default();
        let mut measurements = 0;
        for second in 0..40 {
            let heading = TAU * second as f64 / TURN_SECONDS;
            let east = AIR_SPEED * heading.sin();
            let north = AIR_SPEED * heading.cos();
            if let Some(fit) = update(&mut wind, second as f64, east, north) {
                wind.accept(fit);
                measurements += 1;
            }
        }

        assert_eq!(measurements, 1);
    }
}
