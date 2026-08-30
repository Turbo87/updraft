//! Measures the estimator against a recorded flight.
//!
//! `testdata/weglide_1141558.igc` is a five-hour cross-country flight in a
//! JS-3-18m, logged by an LXNAV LX9070 with a V9 vario. Its B records
//! carry the instrument's own total-energy vario (`VAT`), and its K
//! records the instrument's wind (`WDI`, `WSP`). The instrument derived
//! those from sensors the estimator does not have: a total-energy probe
//! and an inertial platform. The recorded values are therefore a
//! reference to measure against, not a ground truth.
//!
//! Only soaring flight is scored. Scoring excludes engine-running periods,
//! the following settling minute, and samples below the airborne threshold.
//! The estimator still receives every sample so its state matches a full
//! flight replay.
//!
//! The snapshot is a regression guard on estimate quality. A change that
//! moves it has to say which way the numbers moved, and why.

use igc::records::{Extendable, Extension, Record};
use std::fmt::Write as _;
use std::time::Duration;
use updraft_units::{Angle, EllipsoidAltitude, Length, PressureAltitude, Speed};

use super::estimator::{Estimator, Fix};
use super::sample::SampleAcceptance::Accepted;

/// Read at compile time so that the parsed extension definitions can
/// borrow from it across records.
const RECORDING: &str = include_str!("../../../../testdata/weglide_1141558.igc");

/// Engine noise level above which the engine counts as running.
const ENGINE_RUNNING: f64 = 200.;

/// How long the engine has to have been quiet before scoring resumes.
const ENGINE_SETTLING: Duration = Duration::from_secs(60);

/// How far above the lowest pressure altitude so far the glider counts as
/// airborne, in metres.
const AIRBORNE_ALTITUDE_GAIN: f64 = 100.;

#[test]
fn estimates_match_the_recorded_instrument_values() {
    insta::assert_snapshot!(recorded_flight_report(RECORDING));
}

fn recorded_flight_report(recording: &str) -> String {
    let mut report = String::new();
    let header = "quantity           unit        n    rms    mae   bias   corr";
    writeln!(report, "{header}").unwrap();
    write!(report, "{}", measure(recording, AirSpeed::FromSensor)).unwrap();
    writeln!(report, "-- without an airspeed sensor --").unwrap();
    write!(report, "{}", measure(recording, AirSpeed::Withheld)).unwrap();
    report
}

/// Whether the recorded airspeed is passed on, so that the run measures
/// what a device without an airspeed sensor would produce.
#[derive(Clone, Copy, PartialEq)]
enum AirSpeed {
    FromSensor,
    Withheld,
}

fn measure(recording: &str, air_speed: AirSpeed) -> String {
    let mut estimator = Estimator::new();
    let mut vertical_speed = Errors::default();
    let mut wind_speed = Errors::default();
    let mut wind_direction = Errors::default();

    let mut fix_extensions = Vec::new();
    let mut wind_extensions = Vec::new();
    let mut estimated_wind = None;
    let mut quiet_since = None;
    let mut lowest = f64::INFINITY;
    let mut soaring = false;

    for line in recording.lines() {
        match Record::parse_line(line) {
            Ok(Record::I(definition)) => fix_extensions = definition.0.extensions,
            Ok(Record::J(definition)) => wind_extensions = definition.0.extensions,
            Ok(Record::B(record)) => {
                let value = |mnemonic| extension(&record, &fix_extensions, mnemonic);
                let time = seconds(&record.timestamp);
                if air_speed == AirSpeed::FromSensor {
                    let air_speed = hundredths_kmh(value("TAS"));
                    let acceptance = estimator.air_speed(time, air_speed);
                    assert_eq!(acceptance, Accepted);
                }
                let _ = estimator.fix(
                    time,
                    &Fix {
                        track: Angle::from_degrees(value("TRT")),
                        ground_speed: hundredths_kmh(value("GSP")),
                    },
                );
                // A zero GNSS altitude means the recorder had no fix.
                if record.gps_alt != 0 {
                    let altitude = Length::from_meters(f64::from(record.gps_alt));
                    let altitude = EllipsoidAltitude::new(altitude);
                    let acceptance = estimator.gnss_altitude(time, altitude);
                    assert_eq!(acceptance, Accepted);
                }
                let altitude = Length::from_meters(f64::from(record.pressure_alt));
                let altitude = PressureAltitude::new(altitude);
                let acceptance = estimator.pressure_altitude(time, altitude);
                assert_eq!(acceptance, Accepted);

                lowest = lowest.min(f64::from(record.pressure_alt));
                quiet_since = match value("ENL") < ENGINE_RUNNING {
                    true => quiet_since.or(Some(time)),
                    false => None,
                };
                soaring = quiet_since
                    .is_some_and(|since| time.saturating_sub(since) >= ENGINE_SETTLING)
                    && f64::from(record.pressure_alt) >= lowest + AIRBORNE_ALTITUDE_GAIN;

                let estimate = estimator.estimate();
                estimated_wind = estimate.wind;
                if !soaring {
                    continue;
                }
                let estimated = estimate.vario;
                if let Some(estimated) = estimated {
                    vertical_speed.add(estimated.as_meters_per_second(), value("VAT") / 100.);
                }
            }
            Ok(Record::K(record)) => {
                let Some(wind) = estimated_wind.filter(|_| soaring) else {
                    continue;
                };
                let value = |mnemonic| extension(&record, &wind_extensions, mnemonic);
                wind_speed.add(
                    wind.speed.as_meters_per_second(),
                    hundredths_kmh(value("WSP")).as_meters_per_second(),
                );
                wind_direction.add_difference(
                    (wind.direction - Angle::from_degrees(value("WDI")))
                        .normalized_signed()
                        .as_degrees(),
                );
            }
            _ => {}
        }
    }

    let mut rows = String::new();
    writeln!(rows, "total energy       m/s   {}", vertical_speed.row()).unwrap();
    writeln!(rows, "wind speed         m/s   {}", wind_speed.row()).unwrap();
    writeln!(rows, "wind direction     deg   {}", wind_direction.row()).unwrap();
    rows
}

fn seconds(time: &igc::util::Time) -> Duration {
    let seconds = u64::from(time.hours) * 3600 + u64::from(time.minutes) * 60;
    Duration::from_secs(seconds + u64::from(time.seconds))
}

/// LXNAV writes speeds as hundredths of a kilometre per hour.
fn hundredths_kmh(value: f64) -> Speed {
    Speed::from_kilometers_per_hour(value / 100.)
}

/// Reads one numeric extension. The recording defines every extension the
/// estimate needs, so a missing or malformed one is a broken test.
fn extension(record: &impl Extendable, extensions: &[Extension<'_>], mnemonic: &str) -> f64 {
    let extension = extensions
        .iter()
        .find(|extension| extension.mnemonic == mnemonic)
        .unwrap_or_else(|| panic!("the recording defines the {mnemonic} extension"));
    record
        .get_extension(extension)
        .expect("the record is long enough for its defined extensions")
        .parse()
        .unwrap_or_else(|_| panic!("the {mnemonic} extension holds a number"))
}

/// Running error statistics against a reference series.
///
/// Correlation needs both series. Compass directions only have an angular
/// difference, so their correlation is not reported.
#[derive(Default)]
struct Errors {
    count: usize,
    error_sum: f64,
    absolute_sum: f64,
    square_sum: f64,
    correlation: Option<Correlation>,
}

#[derive(Default)]
struct Correlation {
    estimates: f64,
    estimate_squares: f64,
    references: f64,
    reference_squares: f64,
    products: f64,
}

impl Errors {
    fn add(&mut self, estimate: f64, reference: f64) {
        self.add_difference(estimate - reference);

        let correlation = self.correlation.get_or_insert_default();
        correlation.estimates += estimate;
        correlation.estimate_squares += estimate * estimate;
        correlation.references += reference;
        correlation.reference_squares += reference * reference;
        correlation.products += estimate * reference;
    }

    fn add_difference(&mut self, error: f64) {
        self.count += 1;
        self.error_sum += error;
        self.absolute_sum += error.abs();
        self.square_sum += error * error;
    }

    fn row(&self) -> String {
        if self.count == 0 {
            return format!("{:6} {:>6} {:>6} {:>6} {:>6}", 0, "-", "-", "-", "-");
        }
        let count = self.count as f64;
        let correlation = match &self.correlation {
            Some(correlation) => {
                let covariance = correlation.products / count
                    - correlation.estimates * correlation.references / (count * count);
                let estimates =
                    correlation.estimate_squares / count - (correlation.estimates / count).powi(2);
                let references = correlation.reference_squares / count
                    - (correlation.references / count).powi(2);
                format!("{:6.2}", covariance / (estimates * references).sqrt())
            }
            None => "     -".to_owned(),
        };
        format!(
            "{:6} {:6.2} {:6.2} {:+6.2} {correlation}",
            self.count,
            (self.square_sum / count).sqrt(),
            self.absolute_sum / count,
            self.error_sum / count,
        )
    }
}
