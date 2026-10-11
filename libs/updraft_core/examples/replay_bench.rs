//! Replays recorded flights through [`SensorFusion`] and measures what the
//! setups without an airspeed source lose.
//!
//! The Reference scenario uses every recorded input. The other scenarios
//! remove inputs from the same flight. The bench scores each value against
//! the Reference output, so that errors that all scenarios share cancel.
//! Derived airspeed is scored against the recorded TAS. A last section
//! compares the Reference output with the vario and wind that the recorder
//! wrote, as a sanity check.
//!
//! All values are in m/s. Wind takes its bias from the wind speed and its
//! RMS from the vector difference.
//!
//! Run the bench on a directory of IGC files:
//!
//! ```sh
//! cargo run -p updraft_core --example replay_bench -- <directory>
//! ```
//!
//! The output holds only aggregate values, because a collection of
//! recorded flights can hold personal data.

use igc::records::{Extendable, Extension, Record};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::time::Duration;
use updraft_core::{
    DomainState, FusionInputs, GpsSnapshot, Selected, SensorFusion, SourceId, SpeedInstrument,
    Timed, Timestamp,
};
use updraft_geo::LatLon;
use updraft_units::{Angle, EllipsoidAltitude, Length, MslAltitude, PressureAltitude, Speed};

/// Engine noise level above which the engine counts as running.
const ENGINE_RUNNING: f64 = 200.;

/// How long the engine has to have been quiet before scoring resumes.
const ENGINE_SETTLING: Duration = Duration::from_secs(60);

/// How far above the lowest pressure altitude so far the glider counts as
/// airborne.
const AIRBORNE_ALTITUDE_GAIN: Length = Length::from_meters(100.);

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

fn main() {
    let directory = std::env::args_os()
        .nth(1)
        .expect("usage: replay_bench <directory>");
    let mut paths = std::fs::read_dir(directory)
        .expect("the directory is readable")
        .map(|entry| entry.expect("the directory entry is readable").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("igc"))
        })
        .collect::<Vec<_>>();
    paths.sort();

    let mut bench = Bench::default();
    for path in paths {
        bench.add_file(&path);
    }
    print!("{bench}");
}

#[derive(Clone, Copy)]
enum Scenario {
    Reference,
    NoAirspeedSource,
    GpsOnly,
}

const SCENARIOS: [Scenario; 3] = [
    Scenario::Reference,
    Scenario::NoAirspeedSource,
    Scenario::GpsOnly,
];

impl Scenario {
    fn name(self) -> &'static str {
        match self {
            Self::Reference => "reference",
            Self::NoAirspeedSource => "no airspeed source",
            Self::GpsOnly => "gps only",
        }
    }

    fn inputs(self, sample: &Sample) -> FusionInputs {
        let at = sample.time;
        let gps = GpsSnapshot {
            position: sample.position,
            altitude_msl: sample
                .gnss_altitude
                .map(|altitude| Timed::new(altitude, at)),
            track: Some(Timed::new(sample.track, at)),
            ground_speed: Some(Timed::new(sample.ground_speed, at)),
            fix_time: None,
        };
        let true_airspeed = match self {
            Self::Reference => current(sample.true_airspeed, at),
            Self::NoAirspeedSource | Self::GpsOnly => DomainState::Unavailable,
        };
        let pressure_altitude = match self {
            Self::Reference | Self::NoAirspeedSource => current(sample.pressure_altitude, at),
            Self::GpsOnly => DomainState::Unavailable,
        };
        FusionInputs {
            gps: current(gps, at),
            true_airspeed,
            pressure_altitude,
        }
    }
}

fn current<T>(value: T, at: Timestamp) -> DomainState<T> {
    DomainState::Current(Selected {
        source: SourceId::InternalGps,
        ingested_at: at,
        value,
    })
}

/// One B record and the wind of the K record that follows it.
struct Sample {
    time: Timestamp,
    position: LatLon,
    track: Angle,
    ground_speed: Speed,
    true_airspeed: Speed,
    pressure_altitude: PressureAltitude,
    gnss_altitude: Option<MslAltitude>,
    recorded_vario: Option<f64>,
    recorded_wind: Option<Wind>,
    /// Whether the sample is in soaring flight, which the bench scores.
    scored: bool,
}

/// Why the bench does not replay a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Exclusion {
    NoTrueAirspeed,
    NoGroundVelocity,
}

impl fmt::Display for Exclusion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoTrueAirspeed => "no TAS",
            Self::NoGroundVelocity => "no GSP or TRT",
        })
    }
}

/// The B record extensions that the bench reads.
struct FixLayout<'a> {
    true_airspeed: Extension<'a>,
    ground_speed: Extension<'a>,
    track: Extension<'a>,
    engine_noise: Option<Extension<'a>>,
    vario: Option<Extension<'a>>,
}

impl<'a> FixLayout<'a> {
    fn new(extensions: &[Extension<'a>]) -> Result<Self, Exclusion> {
        let find = |mnemonic| {
            extensions
                .iter()
                .find(|extension| extension.mnemonic == mnemonic)
                .cloned()
        };
        Ok(Self {
            true_airspeed: find("TAS").ok_or(Exclusion::NoTrueAirspeed)?,
            ground_speed: find("GSP").ok_or(Exclusion::NoGroundVelocity)?,
            track: find("TRT").ok_or(Exclusion::NoGroundVelocity)?,
            engine_noise: find("ENL"),
            vario: find("VAT"),
        })
    }
}

/// Reads the samples of one recording.
///
/// The bench reads the LXNAV units: hundredths of a kilometre per hour for
/// speeds and centimetres per second for the vario. It skips B records
/// with an unreadable extension.
fn parse(recording: &str) -> Result<Vec<Sample>, Exclusion> {
    let mut layout = Err(Exclusion::NoTrueAirspeed);
    let mut wind_extensions = Vec::new();
    let mut samples = Vec::<Sample>::new();
    let mut day = Duration::ZERO;
    let mut soaring = SoaringWindow::default();

    for line in recording.lines() {
        match Record::parse_line(line) {
            Ok(Record::I(definition)) => layout = FixLayout::new(&definition.0.extensions),
            Ok(Record::J(definition)) => wind_extensions = definition.0.extensions,
            Ok(Record::B(record)) => {
                let layout = layout.as_ref().map_err(|exclusion| *exclusion)?;
                let value = |extension: &Extension<'_>| -> Option<f64> {
                    record.get_extension(extension).ok()?.trim().parse().ok()
                };
                let (Some(true_airspeed), Some(ground_speed), Some(track)) = (
                    value(&layout.true_airspeed),
                    value(&layout.ground_speed),
                    value(&layout.track),
                ) else {
                    continue;
                };

                let mut time =
                    day + Duration::from_secs(record.timestamp.seconds_since_midnight().into());
                if samples
                    .last()
                    .is_some_and(|last| time < last.time.since_start())
                {
                    day += DAY;
                    time += DAY;
                }
                let pressure_altitude = Length::from_meters(f64::from(record.pressure_alt));
                let engine_noise = layout.engine_noise.as_ref().and_then(value);
                let position = LatLon::from_degrees(record.pos.lat.into(), record.pos.lon.into());
                // A zero GNSS altitude means the recorder had no fix.
                let gnss_altitude = (record.gps_alt != 0).then(|| {
                    let altitude = Length::from_meters(f64::from(record.gps_alt));
                    updraft_egm96::ellipsoidal_to_msl(position, EllipsoidAltitude::new(altitude))
                });
                samples.push(Sample {
                    time: Timestamp::from_millis(time.as_millis().try_into().unwrap()),
                    position,
                    track: Angle::from_degrees(track),
                    ground_speed: hundredths_kmh(ground_speed),
                    true_airspeed: hundredths_kmh(true_airspeed),
                    pressure_altitude: PressureAltitude::new(pressure_altitude),
                    gnss_altitude,
                    recorded_vario: layout
                        .vario
                        .as_ref()
                        .and_then(value)
                        .map(|vario| vario / 100.),
                    recorded_wind: None,
                    scored: soaring.observe(time, pressure_altitude, engine_noise),
                });
            }
            Ok(Record::K(record)) => {
                let value = |mnemonic| -> Option<f64> {
                    let extension = wind_extensions
                        .iter()
                        .find(|extension| extension.mnemonic == mnemonic)?;
                    record.get_extension(extension).ok()?.trim().parse().ok()
                };
                let speed = value("WSP").or_else(|| value("WVE"));
                if let (Some(sample), Some(direction), Some(speed)) =
                    (samples.last_mut(), value("WDI"), speed)
                {
                    let direction = Angle::from_degrees(direction);
                    sample.recorded_wind = Some(Wind::new(direction, hundredths_kmh(speed)));
                }
            }
            _ => {}
        }
    }
    Ok(samples)
}

/// Finds the soaring flight: the engine is quiet for the settling time,
/// and the glider is airborne above the lowest altitude so far.
#[derive(Default)]
struct SoaringWindow {
    quiet_since: Option<Duration>,
    lowest: Option<Length>,
}

impl SoaringWindow {
    /// A recording without engine noise has no engine to run.
    fn observe(&mut self, time: Duration, altitude: Length, engine_noise: Option<f64>) -> bool {
        let lowest = match self.lowest {
            Some(lowest) if lowest <= altitude => lowest,
            _ => altitude,
        };
        self.lowest = Some(lowest);
        self.quiet_since = match engine_noise.is_none_or(|noise| noise < ENGINE_RUNNING) {
            true => self.quiet_since.or(Some(time)),
            false => None,
        };
        self.quiet_since
            .is_some_and(|since| time.saturating_sub(since) >= ENGINE_SETTLING)
            && altitude >= lowest + AIRBORNE_ALTITUDE_GAIN
    }
}

fn hundredths_kmh(value: f64) -> Speed {
    Speed::from_kilometers_per_hour(value / 100.)
}

/// The wind vector in m/s towards east and north.
#[derive(Clone, Copy)]
struct Wind {
    east: f64,
    north: f64,
}

impl Wind {
    /// Converts the direction that the wind comes from and its speed.
    fn new(direction: Angle, speed: Speed) -> Self {
        let (sin, cos) = direction.sin_cos();
        let speed = speed.as_meters_per_second();
        Self {
            east: -speed * sin,
            north: -speed * cos,
        }
    }

    fn speed(self) -> f64 {
        self.east.hypot(self.north)
    }
}

/// The values of one scenario that are current after a sample.
#[derive(Clone, Copy, Default)]
struct Outputs {
    vario: Option<f64>,
    vertical_speed: Option<f64>,
    derived_airspeed: Option<f64>,
    wind: Option<Wind>,
}

impl Outputs {
    fn read(fusion: &SensorFusion) -> Self {
        let Some(instruments) = fusion.instruments() else {
            return Self::default();
        };
        let current = |instrument: Option<SpeedInstrument>| {
            instrument
                .filter(|instrument| !instrument.stale)
                .map(|instrument| instrument.meters_per_second)
        };
        Self {
            vario: current(instruments.vario),
            vertical_speed: current(instruments.vertical_speed),
            derived_airspeed: current(instruments.airspeed),
            wind: instruments.wind.filter(|wind| !wind.stale).map(|wind| {
                let direction = Angle::from_degrees(wind.direction_degrees);
                Wind::new(
                    direction,
                    Speed::from_meters_per_second(wind.speed_meters_per_second),
                )
            }),
        }
    }
}

/// Error statistics of one value against its reference.
#[derive(Clone, Copy, Default)]
struct Cell {
    /// Scored samples.
    samples: usize,
    /// Scored samples where the value is current.
    available: usize,
    /// Scored samples where the value and its reference are current.
    compared: usize,
    error_sum: f64,
    square_sum: f64,
}

impl Cell {
    fn add_scalar(&mut self, estimate: Option<f64>, reference: Option<f64>) {
        self.add(estimate, reference, |estimate, reference| {
            let error = estimate - reference;
            (error, error * error)
        });
    }

    /// Takes the bias from the wind speed and the RMS from the vector.
    fn add_wind(&mut self, estimate: Option<Wind>, reference: Option<Wind>) {
        self.add(estimate, reference, |estimate, reference| {
            let east = estimate.east - reference.east;
            let north = estimate.north - reference.north;
            (
                estimate.speed() - reference.speed(),
                east * east + north * north,
            )
        });
    }

    fn add<T: Copy>(
        &mut self,
        estimate: Option<T>,
        reference: Option<T>,
        errors: impl FnOnce(T, T) -> (f64, f64),
    ) {
        self.samples += 1;
        self.available += usize::from(estimate.is_some());
        if let Some((estimate, reference)) = estimate.zip(reference) {
            let (error, square) = errors(estimate, reference);
            self.compared += 1;
            self.error_sum += error;
            self.square_sum += square;
        }
    }

    fn availability(&self) -> String {
        match self.samples {
            0 => "-".to_owned(),
            samples => format!("{:.1}%", 100. * self.available as f64 / samples as f64),
        }
    }

    fn errors(&self) -> String {
        if self.compared == 0 {
            return format!("{:7} {:>6} {:>6}", 0, "-", "-");
        }
        let count = self.compared as f64;
        format!(
            "{:7} {:+6.2} {:6.2}",
            self.compared,
            self.error_sum / count,
            (self.square_sum / count).sqrt(),
        )
    }
}

/// The cells of one scenario.
#[derive(Clone, Copy, Default)]
struct ScenarioCells {
    vario: Cell,
    vertical_speed: Cell,
    derived_airspeed: Cell,
    wind: Cell,
}

impl ScenarioCells {
    fn add(&mut self, estimate: &Outputs, reference: &Outputs, sample: &Sample) {
        let true_airspeed = sample.true_airspeed.as_meters_per_second();
        self.vario.add_scalar(estimate.vario, reference.vario);
        self.vertical_speed
            .add_scalar(estimate.vertical_speed, reference.vario);
        self.derived_airspeed
            .add_scalar(estimate.derived_airspeed, Some(true_airspeed));
        self.wind.add_wind(estimate.wind, reference.wind);
    }

    fn rows(&self) -> [(&'static str, &Cell); 4] {
        [
            ("vario", &self.vario),
            ("vertical speed", &self.vertical_speed),
            ("derived airspeed", &self.derived_airspeed),
            ("wind", &self.wind),
        ]
    }
}

/// Aggregate results over all replayed files.
#[derive(Default)]
struct Bench {
    included: usize,
    excluded: BTreeMap<Exclusion, usize>,
    scenarios: [ScenarioCells; SCENARIOS.len()],
    recorded_vario: Cell,
    recorded_wind: Cell,
}

impl Bench {
    fn add_file(&mut self, path: &Path) {
        let recording = std::fs::read(path).expect("the recording is readable");
        match parse(&String::from_utf8_lossy(&recording)) {
            Ok(samples) => {
                self.included += 1;
                self.replay(&samples);
            }
            Err(exclusion) => *self.excluded.entry(exclusion).or_default() += 1,
        }
    }

    /// Feeds every sample to every scenario, so that the fusion state
    /// matches a full flight, and scores the samples in soaring flight.
    fn replay(&mut self, samples: &[Sample]) {
        let mut fusions = SCENARIOS.map(|_| SensorFusion::default());
        for sample in samples {
            let outputs: [Outputs; SCENARIOS.len()] = std::array::from_fn(|index| {
                fusions[index].update(SCENARIOS[index].inputs(sample));
                Outputs::read(&fusions[index])
            });
            if !sample.scored {
                continue;
            }
            let reference = &outputs[0];
            for (cells, estimate) in self.scenarios.iter_mut().zip(&outputs) {
                cells.add(estimate, reference, sample);
            }
            self.recorded_vario
                .add_scalar(reference.vario, sample.recorded_vario);
            if sample.recorded_wind.is_some() {
                self.recorded_wind
                    .add_wind(reference.wind, sample.recorded_wind);
            }
        }
    }
}

impl fmt::Display for Bench {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let excluded = self.excluded.values().sum::<usize>();
        write!(f, "files: {} included, {excluded} excluded", self.included)?;
        for (exclusion, count) in &self.excluded {
            write!(f, ", {count} {exclusion}")?;
        }
        writeln!(f)?;
        writeln!(f)?;
        writeln!(
            f,
            "scenario            value              avail       n   bias    rms"
        )?;
        for (scenario, cells) in SCENARIOS.iter().zip(&self.scenarios) {
            for (value, cell) in cells.rows() {
                writeln!(
                    f,
                    "{:19} {value:18} {:>6} {}",
                    scenario.name(),
                    cell.availability(),
                    cell.errors(),
                )?;
            }
        }
        writeln!(f)?;
        writeln!(
            f,
            "reference against the recorder           n   bias    rms"
        )?;
        for (value, cell) in [
            ("vario", &self.recorded_vario),
            ("wind", &self.recorded_wind),
        ] {
            writeln!(f, "{value:34} {}", cell.errors())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_fixtures() {
        let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata");
        let mut bench = Bench::default();
        bench.add_file(&testdata.join("weglide_1141558.igc"));
        bench.add_file(&testdata.join("weglide_1015312.igc"));

        insta::assert_snapshot!(bench.to_string());
    }
}
