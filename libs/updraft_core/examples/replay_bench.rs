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
use std::ops::Range;
use std::path::Path;
use std::time::Duration;
use updraft_core::{
    DomainState, FusionInputs, GpsSnapshot, Selected, SensorFusion, SourceId, SpeedInstrument,
    Timed, Timestamp,
};
use updraft_geo::LatLon;
use updraft_polar::{GlidePolar, POLAR_STORE, PolarStoreEntry};
use updraft_units::{Angle, EllipsoidAltitude, Length, MslAltitude, PressureAltitude, Speed};

/// Engine noise level above which the engine counts as running.
const ENGINE_RUNNING: f64 = 200.;

/// How long the engine has to have been quiet before scoring resumes.
const ENGINE_SETTLING: Duration = Duration::from_secs(60);

/// How far above the lowest pressure altitude so far the glider counts as
/// airborne.
const AIRBORNE_ALTITUDE_GAIN: Length = Length::from_meters(100.);

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// Glider types that recorders write differently from the polar store.
const GLIDER_TYPE_ALIASES: [(&str, &str); 3] = [
    ("ASH 25m", "ASH 25E"),
    ("DG-800B", "DG800/18m"),
    ("Duo Discus XLT", "Duo Discus"),
];

/// Track rate below which a sample does not continue a turn, in degrees
/// per second.
const MIN_TURN_RATE: f64 = 3.;

/// How far a turn in one direction has to go to count as circling, in
/// degrees.
const CIRCLING_TURN: f64 = 360.;

/// How long before and after a circling run the samples count as a
/// transition.
const TRANSITION_MARGIN: Duration = Duration::from_secs(20);

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
    phase: Phase,
}

/// The flight phase of a sample, labelled from the recorded track.
///
/// The labels can use later samples, so they are not available to the
/// fusion at the time of the sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Cruise,
    Circling,
    /// Close to a circling run. These samples count only in the cells for
    /// all phases.
    Transition,
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
fn parse(recording: &str) -> Result<Flight, Exclusion> {
    let mut layout = Err(Exclusion::NoTrueAirspeed);
    let mut wind_extensions = Vec::new();
    let mut samples = Vec::<Sample>::new();
    let mut glider_type = None;
    let mut day = Duration::ZERO;
    let mut soaring = SoaringWindow::default();

    for line in recording.lines() {
        match Record::parse_line(line) {
            Ok(Record::H(header)) if header.mnemonic == "GTY" => {
                glider_type = Some(header.data.trim().to_owned());
            }
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
                    phase: Phase::Cruise,
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
    label_phases(&mut samples);
    Ok(Flight {
        polar: glider_type.as_deref().and_then(find_polar),
        samples,
    })
}

struct Flight {
    /// The polar of the recorded glider type at reference mass, without
    /// ballast and bugs.
    polar: Option<GlidePolar>,
    samples: Vec<Sample>,
}

/// Finds the polar after it normalizes case, spaces, and hyphens.
fn find_polar(glider_type: &str) -> Option<GlidePolar> {
    let normalized = |name: &str| -> String {
        name.chars()
            .filter(|character| !matches!(character, ' ' | '-'))
            .flat_map(char::to_lowercase)
            .collect()
    };
    let glider_type = normalized(glider_type);
    let name = GLIDER_TYPE_ALIASES
        .iter()
        .find(|(alias, _)| normalized(alias) == glider_type)
        .map_or(glider_type, |(_, name)| normalized(name));
    POLAR_STORE
        .iter()
        .find(|entry| normalized(entry.name) == name)
        .map(PolarStoreEntry::glide_polar)
}

/// Labels each sample inside a turn in one direction that reaches
/// [`CIRCLING_TURN`] as circling, and the samples within
/// [`TRANSITION_MARGIN`] of such a turn as a transition.
fn label_phases(samples: &mut [Sample]) {
    let runs = circling_runs(samples);
    for run in &runs {
        let start = samples[run.start]
            .time
            .since_start()
            .saturating_sub(TRANSITION_MARGIN);
        let end = samples[run.end - 1].time.since_start() + TRANSITION_MARGIN;
        for sample in samples.iter_mut() {
            if (start..=end).contains(&sample.time.since_start()) {
                sample.phase = Phase::Transition;
            }
        }
    }
    for run in runs {
        for sample in &mut samples[run] {
            sample.phase = Phase::Circling;
        }
    }
}

/// Finds the sample ranges that turn in one direction by at least
/// [`CIRCLING_TURN`].
fn circling_runs(samples: &[Sample]) -> Vec<Range<usize>> {
    let mut runs = Vec::new();
    let mut start = 0;
    let mut direction = 0.;
    let mut turned = 0.;
    for (index, pair) in samples.windows(2).enumerate() {
        let interval = pair[1].time.saturating_since(pair[0].time).as_secs_f64();
        if interval == 0. {
            continue;
        }
        let change = (pair[1].track - pair[0].track)
            .normalized_signed()
            .as_degrees();
        let turning = (change / interval).abs() >= MIN_TURN_RATE;
        if turning && change.signum() == direction {
            turned += change.abs();
            continue;
        }
        if turned >= CIRCLING_TURN {
            runs.push(start..index + 1);
        }
        start = index;
        direction = if turning { change.signum() } else { 0. };
        turned = if turning { change.abs() } else { 0. };
    }
    if turned >= CIRCLING_TURN {
        runs.push(start..samples.len());
    }
    runs
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
    netto: Option<f64>,
    relative_vario: Option<f64>,
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
            netto: current(instruments.netto),
            relative_vario: current(instruments.relative_vario),
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

/// The cells of one scenario and phase.
#[derive(Clone, Copy, Default)]
struct ValueCells {
    vario: Cell,
    vertical_speed: Cell,
    netto: Cell,
    relative_vario: Cell,
    derived_airspeed: Cell,
    wind: Cell,
}

impl ValueCells {
    const COUNT: usize = 6;

    /// Netto and Relative vario count only for a flight with a polar.
    fn add(&mut self, estimate: &Outputs, reference: &Outputs, sample: &Sample, has_polar: bool) {
        let true_airspeed = sample.true_airspeed.as_meters_per_second();
        self.vario.add_scalar(estimate.vario, reference.vario);
        self.vertical_speed
            .add_scalar(estimate.vertical_speed, reference.vario);
        if has_polar {
            self.netto.add_scalar(estimate.netto, reference.netto);
            self.relative_vario
                .add_scalar(estimate.relative_vario, reference.relative_vario);
        }
        self.derived_airspeed
            .add_scalar(estimate.derived_airspeed, Some(true_airspeed));
        self.wind.add_wind(estimate.wind, reference.wind);
    }

    fn rows(&self) -> [(&'static str, &Cell); Self::COUNT] {
        [
            ("vario", &self.vario),
            ("vertical speed", &self.vertical_speed),
            ("netto", &self.netto),
            ("relative vario", &self.relative_vario),
            ("derived airspeed", &self.derived_airspeed),
            ("wind", &self.wind),
        ]
    }
}

/// The cells of one scenario.
#[derive(Clone, Copy, Default)]
struct ScenarioCells {
    cruise: ValueCells,
    circling: ValueCells,
    all: ValueCells,
}

impl ScenarioCells {
    fn add(&mut self, estimate: &Outputs, reference: &Outputs, sample: &Sample, has_polar: bool) {
        self.all.add(estimate, reference, sample, has_polar);
        match sample.phase {
            Phase::Cruise => self.cruise.add(estimate, reference, sample, has_polar),
            Phase::Circling => self.circling.add(estimate, reference, sample, has_polar),
            Phase::Transition => {}
        }
    }

    fn phases(&self) -> [(&'static str, &ValueCells); 3] {
        [
            ("cruise", &self.cruise),
            ("circling", &self.circling),
            ("all", &self.all),
        ]
    }
}

/// Aggregate results over all replayed files.
#[derive(Default)]
struct Bench {
    included: usize,
    without_polar: usize,
    excluded: BTreeMap<Exclusion, usize>,
    scenarios: [ScenarioCells; SCENARIOS.len()],
    recorded_vario: Cell,
    recorded_wind: Cell,
}

impl Bench {
    fn add_file(&mut self, path: &Path) {
        let recording = std::fs::read(path).expect("the recording is readable");
        match parse(&String::from_utf8_lossy(&recording)) {
            Ok(flight) => {
                self.included += 1;
                self.without_polar += usize::from(flight.polar.is_none());
                self.replay(&flight);
            }
            Err(exclusion) => *self.excluded.entry(exclusion).or_default() += 1,
        }
    }

    /// Feeds every sample to every scenario, so that the fusion state
    /// matches a full flight, and scores the samples in soaring flight.
    fn replay(&mut self, flight: &Flight) {
        let mut fusions = SCENARIOS.map(|_| SensorFusion::default());
        if let Some(polar) = flight.polar {
            for fusion in &mut fusions {
                fusion.set_polar(polar);
            }
        }
        for sample in &flight.samples {
            let outputs: [Outputs; SCENARIOS.len()] = std::array::from_fn(|index| {
                fusions[index].update(SCENARIOS[index].inputs(sample));
                Outputs::read(&fusions[index])
            });
            if !sample.scored {
                continue;
            }
            let reference = &outputs[0];
            for (cells, estimate) in self.scenarios.iter_mut().zip(&outputs) {
                cells.add(estimate, reference, sample, flight.polar.is_some());
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
        write!(
            f,
            "files: {} included ({} without a polar), {excluded} excluded",
            self.included, self.without_polar,
        )?;
        for (exclusion, count) in &self.excluded {
            write!(f, ", {count} {exclusion}")?;
        }
        writeln!(f)?;
        writeln!(f)?;
        writeln!(
            f,
            "scenario            value              phase      avail       n   bias    rms"
        )?;
        for (scenario, cells) in SCENARIOS.iter().zip(&self.scenarios) {
            for index in 0..ValueCells::COUNT {
                for (phase, cells) in cells.phases() {
                    let (value, cell) = cells.rows()[index];
                    writeln!(
                        f,
                        "{:19} {value:18} {phase:8} {:>7} {}",
                        scenario.name(),
                        cell.availability(),
                        cell.errors(),
                    )?;
                }
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
