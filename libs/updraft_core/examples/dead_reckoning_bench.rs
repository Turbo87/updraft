//! Replays recorded NMEA flights and compares motion models for dead
//! reckoning of the ownship and traffic symbols.
//!
//! Each model takes the received positions up to one fix and predicts the
//! position 0.5, 1.0, and 1.5 seconds after the fix time. The bench scores
//! the prediction against the position that later fixes show at that time.
//! A Catmull-Rom spline through the later fixes supplies positions between
//! fixes. Traffic movement is also scored against IGC files of the traffic
//! targets, when the directory contains them. The movement comparison
//! removes the offset between the FLARM and IGC positions at the fix time.
//!
//! The ownship uses the RMC fixes. Traffic uses the positions that
//! [`Core`] accepts with the FLARM position correction, at the GPS time of
//! the reference fix. The reported FLARM ground speed and track come from
//! the PFLAA report that supplied the accepted position.
//!
//! The bench matches a target to an IGC file when the accepted positions
//! are close to the IGC positions at the same time. It ignores the dates,
//! so a directory must not hold recordings of different days with the same
//! aircraft.
//!
//! All errors are in meters. The `rms` columns are root-mean-square errors,
//! and the `p50` and `p95` columns are percentiles.
//!
//! Run the bench on a directory of NMEA and IGC files:
//!
//! ```sh
//! cargo run -p updraft_core --example dead_reckoning_bench -- <directory>
//! ```
//!
//! The output holds only aggregate values, because a collection of
//! recorded flights can hold personal data.

use igc::records::Record;
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::TAU;
use std::fmt;
use std::path::Path;
use updraft_core::{
    Bytes, ConnectionSpec, Core, Effect, ExternalDeviceConfig, Settings, SettingsSnapshot,
    Timestamp, Topic, TrafficTargetId, TrafficTargetIdType, TrafficUpdate,
};
use updraft_geo::LatLon;
use updraft_nmea::{Message, RmcStatus, Step};

/// The prediction times after the fix time, in seconds.
const HORIZONS: [f64; 3] = [0.5, 1.0, 1.5];

/// The age limit of the positions that a model uses, in seconds.
const WINDOW: f64 = 5.;

/// The maximum number of positions that a model uses.
const WINDOW_POSITIONS: usize = 5;

/// Ground speed from which the bench scores a fix as flight, in m/s.
const FLIGHT_SPEED: f64 = 15.;

/// Upper limits of the ground speed buckets below flight speed, in m/s.
const SLOW_SPEEDS: [f64; 3] = [2., 5., FLIGHT_SPEED];

/// Turn rate from which a fix counts as turning, in degrees per second.
const TURNING_RATE: f64 = 8.;

/// Turn rate below which a fix counts as straight, in degrees per second.
const STRAIGHT_RATE: f64 = 4.;

/// Change of the turn rate from which a fix counts as a turn entry or
/// exit, in degrees per second.
const TRANSITION_CHANGE: f64 = 6.;

/// Minimum number of shared fixes for a match between a target and an
/// IGC file.
const MATCH_FIXES: usize = 120;

/// Median distance below which a target matches an IGC file, in meters.
const MATCH_DISTANCE: f64 = 50.;

/// Turn rate limits of the limit comparison, in degrees per second.
const TURN_RATE_LIMITS: [Option<f64>; 6] =
    [Some(15.), Some(20.), Some(25.), Some(30.), Some(40.), None];

/// A received position and the motion that the source reported with it.
#[derive(Clone, Copy, Debug)]
struct Fix {
    /// GPS time in seconds since midnight.
    time: f64,
    position: LatLon,
    /// Ground speed in m/s.
    speed: f64,
    /// Reported ground track in radians.
    track: f64,
    /// Track that the core publishes, in radians. For traffic, this is the
    /// bearing between accepted positions.
    published_track: f64,
}

/// A time series of positions, in seconds since midnight.
type TimedPositions = [(f64, LatLon)];

fn main() {
    let directory = std::env::args_os()
        .nth(1)
        .expect("usage: dead_reckoning_bench <directory>");
    let mut paths = std::fs::read_dir(directory)
        .expect("the directory is readable")
        .map(|entry| entry.expect("the directory entry is readable").path())
        .collect::<Vec<_>>();
    paths.sort();

    let mut bench = Bench::default();
    for path in &paths {
        if has_extension(path, "nmea") {
            bench.add_nmea(&std::fs::read(path).expect("the NMEA file is readable"));
        }
    }
    for path in &paths {
        if has_extension(path, "igc") {
            let bytes = std::fs::read(path).expect("the IGC file is readable");
            bench.add_igc(&String::from_utf8_lossy(&bytes));
        }
    }
    print!("{bench}");
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

/// Reads the RMC fixes and the accepted traffic positions of a recording.
fn replay(recording: &[u8]) -> (Vec<Fix>, BTreeMap<String, Vec<Fix>>) {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![ExternalDeviceConfig {
            enabled: true,
            spec: ConnectionSpec::tcp("127.0.0.1", 4353),
        }],
    });
    let device_id = core
        .topics()
        .into_iter()
        .find_map(|topic| match topic {
            Topic::ExternalDevices(devices) => Some(devices[0].device_id),
            _ => None,
        })
        .expect("the core publishes the configured device");

    let mut ownship = Vec::<Fix>::new();
    let mut traffic = BTreeMap::<String, Vec<Fix>>::new();
    let mut gps_time = None;
    let mut first_gps_time = None;
    let mut at = 0;
    for line in recording.split_inclusive(|&byte| byte == b'\n') {
        let mut input = line;
        let report = match updraft_nmea::parse(&mut input) {
            Step::Frame(Message::Rmc(rmc)) => {
                let time = rmc
                    .utc_time
                    .map(|time| f64::from(time.milliseconds_since_midnight()) / 1000.);
                if let (RmcStatus::Active, Some(time), Some(position), Some(speed), Some(track)) = (
                    rmc.status,
                    time,
                    rmc.position,
                    rmc.speed_over_ground,
                    rmc.course_over_ground,
                ) && ownship.last().is_none_or(|last| time > last.time)
                {
                    let track = track.as_radians();
                    ownship.push(Fix {
                        time,
                        position,
                        speed: speed.as_meters_per_second(),
                        track,
                        published_track: track,
                    });
                }
                if let Some(time) = time {
                    gps_time = Some(time);
                    let first = *first_gps_time.get_or_insert(time);
                    // The core times its inputs on the receive clock.
                    at = at.max(((time - first) * 1000.) as u64);
                }
                None
            }
            Step::Frame(Message::Pflaa(pflaa)) => pflaa
                .id_type
                .zip(pflaa.id)
                .zip(pflaa.ground_speed.zip(pflaa.track))
                .map(|((id_type, id), (speed, track))| {
                    let id = TrafficTargetId::new(TrafficTargetIdType::from(id_type), id.address);
                    (
                        id.to_string(),
                        speed.as_meters_per_second(),
                        track.as_radians(),
                    )
                }),
            _ => None,
        };

        at += 1;
        let effects = core
            .apply(Bytes::new(device_id, line), Timestamp::from_millis(at))
            .effects;
        let (Some((id, speed, track)), Some(time)) = (report, gps_time) else {
            continue;
        };
        for effect in effects {
            let Effect::Emit(Topic::Traffic(TrafficUpdate::Delta(delta))) = effect else {
                continue;
            };
            for target in delta.upserts.into_iter().filter(|target| target.id == id) {
                let position = LatLon::from_degrees(
                    target.position.latitude_degrees,
                    target.position.longitude_degrees,
                );
                let fixes = traffic.entry(id.clone()).or_default();
                // Repeated reports at one GPS time keep the accepted position.
                if fixes.last().is_some_and(|last| time <= last.time) || target.stale {
                    continue;
                }
                let Some(published_track) = target.track_degrees else {
                    continue;
                };
                fixes.push(Fix {
                    time,
                    position,
                    speed,
                    track,
                    published_track: published_track.to_radians(),
                });
            }
        }
    }
    (ownship, traffic)
}

/// Reads the B record positions of an IGC file.
fn igc_path(recording: &str) -> Vec<(f64, LatLon)> {
    recording
        .lines()
        .filter_map(|line| match Record::parse_line(line) {
            Ok(Record::B(record)) => Some((
                f64::from(record.timestamp.seconds_since_midnight()),
                LatLon::from_degrees(record.pos.lat.into(), record.pos.lon.into()),
            )),
            _ => None,
        })
        .collect()
}

/// East and north offset of `point` from `origin`, in meters.
fn local(origin: LatLon, point: LatLon) -> (f64, f64) {
    let (distance, bearing) = origin.distance_bearing(point);
    let (sin, cos) = bearing.as_radians().sin_cos();
    (distance.as_meters() * sin, distance.as_meters() * cos)
}

/// The angle in the range -π to π.
fn wrap(angle: f64) -> f64 {
    angle - TAU * (angle / TAU).round()
}

/// The position on `path` at `time`, from a uniform Catmull-Rom spline.
///
/// The spline needs four fixes at one-second intervals around `time`.
fn interpolate(path: &TimedPositions, time: f64, origin: LatLon) -> Option<(f64, f64)> {
    let next = path.partition_point(|&(fix_time, _)| fix_time <= time);
    let start = next.checked_sub(2)?;
    let points = path.get(start..start + 4)?;
    let uniform = points
        .windows(2)
        .all(|pair| (pair[1].0 - pair[0].0 - 1.).abs() < 0.05);
    if !uniform {
        return None;
    }
    let [p0, p1, p2, p3] = [0, 1, 2, 3].map(|index| local(origin, points[index].1));
    let u = time - points[1].0;
    let blend = |a: f64, b: f64, c: f64, d: f64| {
        0.5 * (2. * b
            + (c - a) * u
            + (2. * a - 5. * b + 4. * c - d) * u * u
            + (3. * b - a - 3. * c + d) * u * u * u)
    };
    Some((blend(p0.0, p1.0, p2.0, p3.0), blend(p0.1, p1.1, p2.1, p3.1)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Phase {
    All,
    Straight,
    Turning,
    /// A turn entry or exit.
    Transition,
}

const PHASES: [Phase; 4] = [
    Phase::All,
    Phase::Straight,
    Phase::Turning,
    Phase::Transition,
];

impl Phase {
    /// The phase of the flight around `time`, from the turn rate of the
    /// path before and after `time`.
    ///
    /// The phase uses later fixes, so it is not available to the models.
    /// A fix between the straight and turning limits counts only in `All`.
    fn label(path: &TimedPositions, time: f64, origin: LatLon) -> Option<Self> {
        let points = [-1., 0., 1., 2., 3.].map(|offset| interpolate(path, time + offset, origin));
        let points = points.into_iter().collect::<Option<Vec<_>>>()?;
        let bearings = points
            .windows(2)
            .map(|pair| (pair[1].0 - pair[0].0).atan2(pair[1].1 - pair[0].1))
            .collect::<Vec<_>>();
        let before = wrap(bearings[1] - bearings[0]).to_degrees();
        let after = wrap(bearings[3] - bearings[2]).to_degrees();
        Some(if (after - before).abs() >= TRANSITION_CHANGE {
            Self::Transition
        } else if before.abs().min(after.abs()) >= TURNING_RATE {
            Self::Turning
        } else if before.abs().max(after.abs()) < STRAIGHT_RATE {
            Self::Straight
        } else {
            Self::All
        })
    }

    fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Straight => "straight",
            Self::Turning => "turning",
            Self::Transition => "turn entry or exit",
        }
    }
}

/// A constant ground speed and turn rate from the latest position.
#[derive(Clone, Copy, Debug)]
struct Motion {
    /// Ground speed in m/s.
    speed: f64,
    /// Ground track in radians.
    track: f64,
    /// Turn rate in radians per second, clockwise positive.
    turn_rate: f64,
}

impl Motion {
    /// East and north offset after `seconds` along a circular arc.
    fn advance(self, seconds: f64, limit: Option<f64>) -> (f64, f64) {
        let turn_rate = match limit {
            Some(limit) => self
                .turn_rate
                .clamp(-limit.to_radians(), limit.to_radians()),
            None => self.turn_rate,
        };
        let end = self.track + turn_rate * seconds;
        if turn_rate.abs() < 1e-9 {
            let distance = self.speed * seconds;
            return (distance * self.track.sin(), distance * self.track.cos());
        }
        let radius = self.speed / turn_rate;
        (
            radius * (self.track.cos() - end.cos()),
            radius * (end.sin() - self.track.sin()),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Model {
    /// No dead reckoning.
    Hold,
    /// The reported speed and track.
    Straight,
    /// The reported speed and the published track.
    PublishedStraight,
    /// The turn rate from the reported tracks, as an exponential average
    /// of the track changes with this time constant in seconds. Zero uses
    /// only the latest change.
    TrackRate(u8),
    /// The turn rate from the difference between the reported track and the
    /// bearing of the latest step. On a circle, the bearing lags the track
    /// by half the turn of the step.
    StepRate,
    /// A circle through the latest three positions.
    ThreePointCircle,
    /// A least-squares circle through all positions of the window.
    FittedCircle,
    /// The reported speed and track with the turn rate of the fitted
    /// circle.
    CircleRate,
    /// A quadratic least-squares polynomial in time through all positions
    /// of the window, as a spline extrapolation baseline.
    Quadratic,
}

const MODELS: [Model; 11] = [
    Model::Hold,
    Model::Straight,
    Model::PublishedStraight,
    Model::TrackRate(0),
    Model::TrackRate(1),
    Model::TrackRate(2),
    Model::StepRate,
    Model::ThreePointCircle,
    Model::FittedCircle,
    Model::CircleRate,
    Model::Quadratic,
];

/// The models of the turn rate limit comparison.
const LIMITED_MODELS: [Model; 4] = [
    Model::TrackRate(0),
    Model::TrackRate(1),
    Model::FittedCircle,
    Model::CircleRate,
];

impl fmt::Display for Model {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hold => write!(formatter, "hold"),
            Self::Straight => write!(formatter, "straight"),
            Self::PublishedStraight => write!(formatter, "straight, published track"),
            Self::TrackRate(0) => write!(formatter, "track rate"),
            Self::TrackRate(seconds) => write!(formatter, "track rate, {seconds} s average"),
            Self::ThreePointCircle => write!(formatter, "3-point circle"),
            Self::FittedCircle => write!(formatter, "fitted circle"),
            Self::CircleRate => write!(formatter, "circle turn rate"),
            Self::StepRate => write!(formatter, "step turn rate"),
            Self::Quadratic => write!(formatter, "quadratic"),
        }
    }
}

impl Model {
    /// The motion at the latest fix of `window`, if the model has one.
    fn motion(self, window: &[Fix]) -> Option<Motion> {
        let latest = window.last()?;
        let straight = Motion {
            speed: latest.speed,
            track: latest.track,
            turn_rate: 0.,
        };
        match self {
            Self::Hold | Self::Quadratic => None,
            Self::Straight => Some(straight),
            Self::PublishedStraight => Some(Motion {
                track: latest.published_track,
                ..straight
            }),
            Self::TrackRate(time_constant) => {
                let (sum, weights) = window.windows(2).fold((0., 0.), |(sum, weights), pair| {
                    let rate = wrap(pair[1].track - pair[0].track) / (pair[1].time - pair[0].time);
                    let weight = match time_constant {
                        0 if pair[1].time < latest.time => 0.,
                        0 => 1.,
                        _ => (-(latest.time - pair[1].time) / f64::from(time_constant)).exp(),
                    };
                    (sum + weight * rate, weights + weight)
                });
                let turn_rate = if weights > 0. { sum / weights } else { 0. };
                Some(Motion {
                    turn_rate,
                    ..straight
                })
            }
            Self::ThreePointCircle => {
                Some(circle_motion(&window[window.len().saturating_sub(3)..]).unwrap_or(straight))
            }
            Self::FittedCircle => Some(circle_motion(window).unwrap_or(straight)),
            Self::StepRate => {
                let turn_rate = window.len().checked_sub(2).map_or(0., |previous| {
                    let previous = window[previous];
                    let bearing = previous.position.bearing(latest.position).as_radians();
                    2. * wrap(latest.track - bearing) / (latest.time - previous.time)
                });
                Some(Motion {
                    turn_rate,
                    ..straight
                })
            }
            Self::CircleRate => Some(Motion {
                turn_rate: circle_motion(window).map_or(0., |circle| circle.turn_rate),
                ..straight
            }),
        }
    }

    /// East and north offset from the latest fix of `window` after
    /// `seconds`.
    fn predict(self, window: &[Fix], seconds: f64, limit: Option<f64>) -> (f64, f64) {
        match self {
            Self::Quadratic => quadratic(window, seconds)
                .unwrap_or_else(|| Self::Straight.predict(window, seconds, limit)),
            _ => self
                .motion(window)
                .map_or((0., 0.), |motion| motion.advance(seconds, limit)),
        }
    }
}

/// The motion at the latest fix along a least-squares circle through the
/// fixes, with the reported speed of the latest fix.
fn circle_motion(window: &[Fix]) -> Option<Motion> {
    let latest = window.last()?;
    if window.len() < 3 {
        return None;
    }
    let points = window
        .iter()
        .map(|fix| local(latest.position, fix.position))
        .collect::<Vec<_>>();
    let count = points.len() as f64;
    let mean_x = points.iter().map(|point| point.0).sum::<f64>() / count;
    let mean_y = points.iter().map(|point| point.1).sum::<f64>() / count;
    // Kåsa fit on centered coordinates: x² + y² + D x + E y + F = 0.
    let (mut xx, mut xy, mut yy, mut zx, mut zy, mut z) = (0., 0., 0., 0., 0., 0.);
    for &(x, y) in &points {
        let (x, y) = (x - mean_x, y - mean_y);
        let squared = x * x + y * y;
        xx += x * x;
        xy += x * y;
        yy += y * y;
        zx += squared * x;
        zy += squared * y;
        z += squared;
    }
    let determinant = xx * yy - xy * xy;
    let d = (-zx * yy + zy * xy) / determinant;
    let e = (-zy * xx + zx * xy) / determinant;
    let f = -z / count;
    let radius = (d * d / 4. + e * e / 4. - f).sqrt();
    let center = (mean_x - d / 2., mean_y - e / 2.);
    if !radius.is_finite() || radius > 10_000. {
        return None;
    }
    // The tangent points along the last step. The latest point is the origin.
    let radial = (-center.0, -center.1);
    let step = points[points.len() - 2];
    let clockwise = if radial.0 * step.1 - radial.1 * step.0 >= 0. {
        1.
    } else {
        -1.
    };
    let tangent = (clockwise * radial.1, -clockwise * radial.0);
    Some(Motion {
        speed: latest.speed,
        track: tangent.0.atan2(tangent.1),
        turn_rate: clockwise * latest.speed / radius,
    })
}

/// East and north offset after `seconds` from a quadratic least-squares
/// fit of the positions over time.
fn quadratic(window: &[Fix], seconds: f64) -> Option<(f64, f64)> {
    let latest = window.last()?;
    if window.len() < 3 {
        return None;
    }
    // Normal equations of a + b t + c t² with t relative to the latest fix.
    let mut matrix = [[0.; 3]; 3];
    let mut east = [0.; 3];
    let mut north = [0.; 3];
    for fix in window {
        let t = fix.time - latest.time;
        let powers = [1., t, t * t];
        let (x, y) = local(latest.position, fix.position);
        for row in 0..3 {
            for column in 0..3 {
                matrix[row][column] += powers[row] * powers[column];
            }
            east[row] += powers[row] * x;
            north[row] += powers[row] * y;
        }
    }
    let evaluate = |coefficients: [f64; 3]| {
        coefficients[0] + coefficients[1] * seconds + coefficients[2] * seconds * seconds
    };
    Some((
        evaluate(solve(matrix, east)?),
        evaluate(solve(matrix, north)?),
    ))
}

/// Solves a 3×3 linear system with Cramer's rule.
fn solve(matrix: [[f64; 3]; 3], values: [f64; 3]) -> Option<[f64; 3]> {
    let determinant = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let total = determinant(matrix);
    if total.abs() < 1e-9 {
        return None;
    }
    Some([0, 1, 2].map(|column| {
        let mut replaced = matrix;
        for row in 0..3 {
            replaced[row][column] = values[row];
        }
        determinant(replaced) / total
    }))
}

/// Prediction errors in meters.
#[derive(Default)]
struct Cell(Vec<f64>);

impl Cell {
    fn rms(&self) -> f64 {
        (self.0.iter().map(|error| error * error).sum::<f64>() / self.0.len() as f64).sqrt()
    }

    fn percentile(&self, fraction: f64) -> f64 {
        let mut sorted = self.0.clone();
        sorted.sort_by(f64::total_cmp);
        let rank = (fraction * sorted.len() as f64).ceil() as usize;
        sorted[rank.clamp(1, sorted.len()) - 1]
    }
}

/// The scores of one kind of truth.
#[derive(Default)]
struct Scores {
    /// Errors by phase, model, and horizon index.
    errors: BTreeMap<(Phase, Model, usize), Cell>,
    /// Errors by phase, model, and limit index, at the last horizon.
    limited: BTreeMap<(Phase, Model, usize), Cell>,
    /// Errors at the first full second by speed bucket and model.
    slow: BTreeMap<(usize, Model), Cell>,
    /// Absolute turn rates while turning by model, in degrees per second.
    turn_rates: BTreeMap<Model, Cell>,
    /// Distances between the received position and the truth.
    received: Cell,
}

impl Scores {
    /// Scores every fix of `fixes` against the position on `truth`.
    ///
    /// With `displacement`, the bench compares the predicted movement from
    /// the received position with the movement on `truth` from the fix
    /// time. This removes a constant offset between the two sources.
    fn add(&mut self, fixes: &[Fix], truth: &TimedPositions, displacement: bool) {
        for (index, latest) in fixes.iter().enumerate() {
            let window = &fixes[index.saturating_sub(WINDOW_POSITIONS - 1)..=index];
            let start = window.partition_point(|fix| latest.time - fix.time >= WINDOW);
            let window = &window[start..];
            let origin = latest.position;
            let received = interpolate(truth, latest.time, origin);
            let start = match (displacement, received) {
                (false, _) => (0., 0.),
                (true, Some(received)) => received,
                (true, None) => continue,
            };
            let truths = HORIZONS.map(|horizon| {
                let (east, north) = interpolate(truth, latest.time + horizon, origin)?;
                Some((east - start.0, north - start.1))
            });

            if latest.speed < FLIGHT_SPEED {
                let bucket = SLOW_SPEEDS.partition_point(|&limit| limit <= latest.speed);
                if let Some(truth) = truths[1] {
                    for model in [Model::Hold, Model::Straight] {
                        let error = distance(model.predict(window, HORIZONS[1], None), truth);
                        self.slow.entry((bucket, model)).or_default().0.push(error);
                    }
                }
                continue;
            }
            if let Some(received) = received {
                self.received.0.push(distance((0., 0.), received));
            }
            let Some(phase) = Phase::label(truth, latest.time, origin) else {
                continue;
            };
            let phases = [Phase::All, phase];
            let phases = &phases[..if phase == Phase::All { 1 } else { 2 }];
            for model in MODELS {
                for (horizon_index, (&horizon, truth)) in HORIZONS.iter().zip(truths).enumerate() {
                    let Some(truth) = truth else { continue };
                    let error = distance(model.predict(window, horizon, None), truth);
                    record(&mut self.errors, phases, model, horizon_index, error);
                }
                if phase == Phase::Turning
                    && let Some(motion) = model.motion(window)
                    && model != Model::Hold
                {
                    let rate = motion.turn_rate.to_degrees().abs();
                    self.turn_rates.entry(model).or_default().0.push(rate);
                }
            }
            let (Some(truth), Some(&horizon)) = (truths[2], HORIZONS.last()) else {
                continue;
            };
            for model in LIMITED_MODELS {
                for (limit_index, &limit) in TURN_RATE_LIMITS.iter().enumerate() {
                    let error = distance(model.predict(window, horizon, limit), truth);
                    record(&mut self.limited, phases, model, limit_index, error);
                }
            }
        }
    }
}

/// Adds `error` to the cell of each phase in `phases`.
fn record(
    cells: &mut BTreeMap<(Phase, Model, usize), Cell>,
    phases: &[Phase],
    model: Model,
    index: usize,
    error: f64,
) {
    for &phase in phases {
        cells
            .entry((phase, model, index))
            .or_default()
            .0
            .push(error);
    }
}

fn distance(predicted: (f64, f64), truth: (f64, f64)) -> f64 {
    (predicted.0 - truth.0).hypot(predicted.1 - truth.1)
}

#[derive(Default)]
struct Bench {
    ownship: Scores,
    traffic: Scores,
    traffic_igc: Scores,
    /// Accepted traffic positions of all recordings by target.
    targets: Vec<Vec<Fix>>,
    /// Indices of the targets that match an IGC file.
    matched_targets: BTreeSet<usize>,
    ownship_fixes: usize,
    traffic_fixes: usize,
}

impl Bench {
    fn add_nmea(&mut self, recording: &[u8]) {
        let (ownship, traffic) = replay(recording);
        let path = |fixes: &[Fix]| {
            fixes
                .iter()
                .map(|fix| (fix.time, fix.position))
                .collect::<Vec<_>>()
        };
        self.ownship.add(&ownship, &path(&ownship), false);
        self.ownship_fixes += ownship.len();
        for fixes in traffic.into_values() {
            self.traffic.add(&fixes, &path(&fixes), false);
            self.traffic_fixes += fixes.len();
            self.targets.push(fixes);
        }
    }

    /// Scores the traffic target that matches the IGC file best.
    ///
    /// A target matches at most one IGC file.
    fn add_igc(&mut self, recording: &str) {
        let path = igc_path(recording);
        let matched = self
            .targets
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.matched_targets.contains(index))
            .filter_map(|(index, fixes)| {
                let offsets = Cell(
                    fixes
                        .iter()
                        .filter(|fix| fix.speed >= FLIGHT_SPEED)
                        .filter_map(|fix| interpolate(&path, fix.time, fix.position))
                        .map(|offset| distance((0., 0.), offset))
                        .collect(),
                );
                if offsets.0.len() < MATCH_FIXES {
                    return None;
                }
                let median = offsets.percentile(0.5);
                (median < MATCH_DISTANCE).then_some((median, index))
            })
            .min_by(|first, second| first.0.total_cmp(&second.0));
        if let Some((_, index)) = matched {
            self.traffic_igc.add(&self.targets[index], &path, true);
            self.matched_targets.insert(index);
        }
    }
}

impl fmt::Display for Scores {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for phase in PHASES {
            let samples = self
                .errors
                .get(&(phase, Model::Hold, 1))
                .map_or(0, |cell| cell.0.len());
            writeln!(
                formatter,
                "\n### {} ({samples} fixes at 1.0 s)\n",
                phase.name()
            )?;
            writeln!(
                formatter,
                "{:<27} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
                "model",
                "0.5 s rms",
                "1.0 s rms",
                "1.0 s p50",
                "1.0 s p95",
                "1.5 s rms",
                "1.5 s p95"
            )?;
            for model in MODELS {
                let cell = |horizon| self.errors.get(&(phase, model, horizon));
                let (Some(half), Some(one), Some(one_and_half)) = (cell(0), cell(1), cell(2))
                else {
                    continue;
                };
                writeln!(
                    formatter,
                    "{:<27} {:>9.2} {:>9.2} {:>9.2} {:>9.2} {:>9.2} {:>9.2}",
                    model.to_string(),
                    half.rms(),
                    one.rms(),
                    one.percentile(0.5),
                    one.percentile(0.95),
                    one_and_half.rms(),
                    one_and_half.percentile(0.95),
                )?;
            }
        }

        writeln!(formatter, "\n### turn rate while turning, in °/s\n")?;
        writeln!(
            formatter,
            "{:<27} {:>9} {:>9} {:>9} {:>9}",
            "model", "p50", "p95", "p99", "max"
        )?;
        for (model, cell) in &self.turn_rates {
            if *model == Model::Straight || *model == Model::PublishedStraight {
                continue;
            }
            writeln!(
                formatter,
                "{:<27} {:>9.1} {:>9.1} {:>9.1} {:>9.1}",
                model.to_string(),
                cell.percentile(0.5),
                cell.percentile(0.95),
                cell.percentile(0.99),
                cell.percentile(1.),
            )?;
        }

        writeln!(formatter, "\n### 1.5 s rms by turn rate limit in °/s\n")?;
        write!(formatter, "{:<36}", "model")?;
        for limit in TURN_RATE_LIMITS {
            let limit = limit.map_or("none".to_owned(), |limit| format!("{limit}"));
            write!(formatter, " {limit:>6}")?;
        }
        writeln!(formatter)?;
        for phase in [Phase::All, Phase::Turning] {
            for model in LIMITED_MODELS {
                write!(formatter, "{:<36}", format!("{model}, {}", phase.name()))?;
                for limit_index in 0..TURN_RATE_LIMITS.len() {
                    let rms = self
                        .limited
                        .get(&(phase, model, limit_index))
                        .map_or(f64::NAN, Cell::rms);
                    write!(formatter, " {rms:>6.2}")?;
                }
                writeln!(formatter)?;
            }
        }

        writeln!(formatter, "\n### 1.0 s rms below flight speed\n")?;
        writeln!(
            formatter,
            "{:<12} {:>9} {:>9} {:>9}",
            "speed", "fixes", "hold", "straight"
        )?;
        let mut lower = 0.;
        for (bucket, upper) in SLOW_SPEEDS.into_iter().enumerate() {
            let cell = |model| self.slow.get(&(bucket, model));
            if let (Some(hold), Some(straight)) = (cell(Model::Hold), cell(Model::Straight)) {
                writeln!(
                    formatter,
                    "{:<12} {:>9} {:>9.2} {:>9.2}",
                    format!("{lower}-{upper} m/s"),
                    hold.0.len(),
                    hold.rms(),
                    straight.rms(),
                )?;
            }
            lower = upper;
        }
        Ok(())
    }
}

impl fmt::Display for Bench {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "## ownship against later fixes")?;
        writeln!(formatter, "\n{} fixes", self.ownship_fixes)?;
        write!(formatter, "{}", self.ownship)?;
        writeln!(formatter, "\n## traffic against later accepted positions")?;
        writeln!(
            formatter,
            "\n{} accepted positions of {} targets",
            self.traffic_fixes,
            self.targets.len()
        )?;
        write!(formatter, "{}", self.traffic)?;
        if !self.matched_targets.is_empty() {
            writeln!(formatter, "\n## traffic movement against IGC files")?;
            writeln!(
                formatter,
                "\n{} matched targets, received position rms {:.2}, p50 {:.2}, p95 {:.2}",
                self.matched_targets.len(),
                self.traffic_igc.received.rms(),
                self.traffic_igc.received.percentile(0.5),
                self.traffic_igc.received.percentile(0.95),
            )?;
            write!(formatter, "{}", self.traffic_igc)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::assert_none;

    #[test]
    fn a_full_turn_returns_to_the_start() {
        let motion = Motion {
            speed: 30.,
            track: 1.,
            turn_rate: TAU / 20.,
        };
        let (east, north) = motion.advance(20., None);
        assert_abs_diff_eq!(east, 0., epsilon = 1e-9);
        assert_abs_diff_eq!(north, 0., epsilon = 1e-9);
    }

    #[test]
    fn a_limited_quarter_turn_ends_on_the_smaller_circle() {
        let motion = Motion {
            speed: 10.,
            track: 0.,
            turn_rate: 1.,
        };
        // 0.5 rad/s for π seconds turns 90° on a 20 m radius to the right.
        let (east, north) = motion.advance(std::f64::consts::PI, Some(0.5_f64.to_degrees()));
        assert_abs_diff_eq!(east, 20., epsilon = 1e-9);
        assert_abs_diff_eq!(north, 20., epsilon = 1e-9);
    }

    #[test]
    fn circle_models_recover_a_steady_turn() {
        let center = LatLon::from_degrees(50., 6.);
        let (speed, radius) = (30., 80.);
        let turn_rate = speed / radius;
        let fixes = (0..5)
            .map(|second| {
                let time = f64::from(second);
                // A right turn around `center`, starting due west of it.
                let bearing = updraft_units::Angle::from_radians(-TAU / 4. + turn_rate * time);
                let position =
                    center.destination(bearing, updraft_units::Length::from_meters(radius));
                Fix {
                    time,
                    position,
                    speed,
                    track: 0.,
                    published_track: 0.,
                }
            })
            .collect::<Vec<_>>();
        for model in [Model::ThreePointCircle, Model::FittedCircle] {
            let motion = model.motion(&fixes).expect("a circle has motion");
            let track = wrap(turn_rate * 4.);
            assert_abs_diff_eq!(motion.turn_rate, turn_rate, epsilon = 1e-3);
            assert_abs_diff_eq!(motion.track, track, epsilon = 1e-3);
        }
    }

    #[test]
    fn interpolation_passes_through_the_fixes() {
        let origin = LatLon::from_degrees(50., 6.);
        let path = (0..4)
            .map(|second| {
                let distance = updraft_units::Length::from_meters(30. * f64::from(second));
                let position = origin.destination(updraft_units::Angle::ZERO, distance);
                (f64::from(second), position)
            })
            .collect::<Vec<_>>();
        let (east, north) = interpolate(&path, 1.5, origin).expect("a uniform path");
        assert_abs_diff_eq!(east, 0., epsilon = 1e-6);
        assert_abs_diff_eq!(north, 45., epsilon = 1e-6);
        assert_none!(interpolate(&path, 0.5, origin));
    }

    #[test]
    fn committed_fixture() {
        let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata");
        let mut bench = Bench::default();
        let recording = std::fs::read(testdata.join("flight_1.nmea")).expect("the fixture exists");
        bench.add_nmea(&recording);

        insta::assert_snapshot!(bench.to_string());
    }
}
