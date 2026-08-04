use crate::smoothing_weight;
use std::collections::VecDeque;
use std::f64::consts::TAU;
use updraft_units::Angle;

/// Shortest gap between buffered samples, in seconds. It keeps the buffer
/// spanning the same amount of *time* whatever rate the fixes arrive at.
const MIN_SPACING: f64 = 0.5;

/// Longest history the fit looks back over, in seconds. A circle that
/// takes longer than this is not a thermalling circle.
const MAX_WINDOW: f64 = 60.;

/// Samples the buffer holds, enough for [`MAX_WINDOW`] at [`MIN_SPACING`].
const CAPACITY: usize = 128;

/// Fewest samples a fit needs.
const MIN_SAMPLES: usize = 8;

/// Airspeeds a circling glider can plausibly fly, in m/s. A fitted radius
/// outside this is a turn that is not a circle.
const MIN_AIR_SPEED: f64 = 15.;
const MAX_AIR_SPEED: f64 = 70.;

/// Largest scatter around the fitted circle that is still accepted, in m/s.
const MAX_RESIDUAL: f64 = 3.;

/// Time constant for smoothing successive fits, in seconds.
const SMOOTHING_TIME_CONSTANT: f64 = 3.;

/// Estimates the wind from the shape of a circle, without an airspeed
/// measurement.
///
/// Through a full circle at a steady airspeed, the ground velocity traces
/// a circle in velocity space. Its centre is the wind and its radius is
/// the airspeed. Fitting all three leaves the estimate weaker than the
/// airspeed-driven [`WindFilter`](crate::wind::WindFilter), which knows
/// the radius and only has to place the centre, and it only updates while
/// the glider circles. It is what remains when no airspeed sensor is
/// connected.
#[derive(Clone, Debug, Default)]
pub struct CirclingWind {
    samples: VecDeque<Sample>,
    /// Track of the newest sample, unwrapped so that a full turn shows as
    /// a change of 2π rather than wrapping around.
    track: f64,
    estimate: Option<Estimate>,
}

#[derive(Clone, Copy, Debug)]
struct Sample {
    time: f64,
    east: f64,
    north: f64,
    track: f64,
}

#[derive(Clone, Copy, Debug)]
struct Estimate {
    east: f64,
    north: f64,
    air_speed: f64,
}

impl CirclingWind {
    /// Folds one ground velocity into the estimate. `time` is the sample's
    /// own time in seconds; it only has to be consistent across calls.
    pub fn update(&mut self, time: f64, east: f64, north: f64) {
        let Some(track) = unwrapped_track(east, north, self.track) else {
            return;
        };
        self.track = track;
        self.push(Sample {
            time,
            east,
            north,
            track,
        });

        let Some(start) = self.completed_turn() else {
            return;
        };
        let points: Vec<_> = self
            .samples
            .iter()
            .skip(start)
            .map(|sample| (sample.east, sample.north))
            .collect();
        let Some(fit) = fit_circle(&points) else {
            return;
        };

        let interval = time - self.samples[start].time;
        let weight = self.estimate.is_some().then(|| {
            let elapsed = (interval / (points.len() - 1) as f64).max(MIN_SPACING);
            smoothing_weight(elapsed, SMOOTHING_TIME_CONSTANT)
        });
        let estimate = self.estimate.get_or_insert(fit);
        if let Some(weight) = weight {
            estimate.east += weight * (fit.east - estimate.east);
            estimate.north += weight * (fit.north - estimate.north);
            estimate.air_speed += weight * (fit.air_speed - estimate.air_speed);
        }
    }

    /// The wind vector in m/s towards east and north, or `None` until a
    /// circle has been flown.
    pub fn vector(&self) -> Option<(f64, f64)> {
        self.estimate.map(|e| (e.east, e.north))
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
    fn completed_turn(&self) -> Option<usize> {
        let track = self.samples.back()?.track;
        (0..self.samples.len().checked_sub(MIN_SAMPLES)?)
            .rev()
            .find(|&index| (track - self.samples[index].track).abs() >= TAU)
    }
}

/// The track the velocity points along, continued from `previous` instead
/// of wrapping. `None` while the glider is too slow for a track.
fn unwrapped_track(east: f64, north: f64, previous: f64) -> Option<f64> {
    if east.hypot(north) < 1. {
        return None;
    }
    let change = Angle::from_radians(east.atan2(north) - previous).normalized_signed();
    Some(previous + change.as_radians())
}

/// Fits a circle through velocity-space points, by the algebraic
/// (Kåsa) method on points shifted to their own mean.
fn fit_circle(points: &[(f64, f64)]) -> Option<Estimate> {
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
    if !(MIN_AIR_SPEED..=MAX_AIR_SPEED).contains(&air_speed) {
        return None;
    }

    let east = mean_east + east;
    let north = mean_north + north;
    let residual = (points
        .iter()
        .map(|&(e, n)| ((e - east).hypot(n - north) - air_speed).powi(2))
        .sum::<f64>()
        / count)
        .sqrt();
    (residual <= MAX_RESIDUAL).then_some(Estimate {
        east,
        north,
        air_speed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some};

    /// Circles for `seconds` at `AIR_SPEED` through a wind of
    /// `(east, north)` m/s, one sample per second.
    fn circle(seconds: usize, east: f64, north: f64) -> CirclingWind {
        const TURN_SECONDS: f64 = 20.;
        const AIR_SPEED: f64 = 30.;

        let mut wind = CirclingWind::default();
        for second in 0..seconds {
            let heading = TAU * second as f64 / TURN_SECONDS;
            wind.update(
                second as f64,
                east + AIR_SPEED * heading.sin(),
                north + AIR_SPEED * heading.cos(),
            );
        }
        wind
    }

    #[test]
    fn a_full_circle_gives_the_wind() {
        let (east, north) = assert_some!(circle(30, -6., -8.).vector());

        assert_abs_diff_eq!(east, -6., epsilon = 0.2);
        assert_abs_diff_eq!(north, -8., epsilon = 0.2);
    }

    #[test]
    fn less_than_a_circle_gives_nothing() {
        assert_none!(circle(15, -6., -8.).vector());
    }

    #[test]
    fn a_straight_glide_gives_nothing() {
        let mut wind = CirclingWind::default();
        for second in 0..120 {
            wind.update(second as f64, 40., 3.);
        }

        assert_none!(wind.vector());
    }

    #[test]
    fn the_estimate_holds_after_the_glider_stops_circling() {
        let mut wind = circle(30, -6., -8.);
        for second in 30..150 {
            wind.update(second as f64, 40., 3.);
        }

        let (east, _) = assert_some!(wind.vector());
        assert_abs_diff_eq!(east, -6., epsilon = 0.2);
    }

    #[test]
    fn time_running_backwards_starts_a_new_buffer() {
        let mut wind = circle(30, -6., -8.);
        // A second circle timed before the first must not be fitted
        // together with it.
        for second in 0..15 {
            let heading = TAU * second as f64 / 20.;
            wind.update(second as f64, 30. * heading.sin(), 30. * heading.cos());
        }

        let (east, _) = assert_some!(wind.vector());
        assert_abs_diff_eq!(east, -6., epsilon = 0.2);
    }

    #[test]
    fn a_faster_fix_rate_gives_the_same_answer() {
        const TURN_SECONDS: f64 = 20.;
        const AIR_SPEED: f64 = 30.;

        let mut wind = CirclingWind::default();
        for tenth in 0..300 {
            let time = tenth as f64 / 10.;
            let heading = TAU * time / TURN_SECONDS;
            wind.update(
                time,
                -6. + AIR_SPEED * heading.sin(),
                -8. + AIR_SPEED * heading.cos(),
            );
        }

        let (east, north) = assert_some!(wind.vector());
        assert_abs_diff_eq!(east, -6., epsilon = 0.2);
        assert_abs_diff_eq!(north, -8., epsilon = 0.2);
    }
}
