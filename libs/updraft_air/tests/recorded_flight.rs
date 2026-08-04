//! Measures the estimator against a recorded flight.
//!
//! `testdata/weglide_1141558.igc` is a five-hour cross-country flight in a
//! JS-3-18m, logged by an LXNAV LX9070 with a V9 vario. Its B records
//! carry the instrument's own total-energy vario (`VAT`) and netto
//! (`NET`), and its K records the instrument's wind (`WDI`, `WSP`). The
//! instrument derived those from sensors the estimator does not have: a
//! total-energy probe and an inertial platform. Its netto also depends on
//! the direction of turn, which no sink rate can. The recorded values are
//! therefore a reference to measure against, not a ground truth.
//!
//! The snapshot is a regression guard on estimate quality. A change that
//! moves it has to say which way the numbers moved, and why.

use igc::records::{Extendable, Extension, Record};
use std::fmt::Write as _;
use std::time::Duration;
use updraft_air::{AirStateEstimator, Sample};
use updraft_polar::{GlidePolar, POLAR_STORE};
use updraft_units::{Angle, EllipsoidAltitude, Length, PressureAltitude, Speed};

/// Read at compile time so that the parsed extension definitions can
/// borrow from it across records.
const RECORDING: &str = include_str!("../../../testdata/weglide_1141558.igc");

/// The glider type in the recording's `HFGTY` header. The recording does
/// not state the flying mass, so the estimate uses the polar's reference
/// mass and understates the sink rate of a ballasted glider.
const GLIDER_TYPE: &str = "JS-3-18m";

#[test]
fn estimates_match_the_recorded_instrument_values() {
    let mut estimator = AirStateEstimator::new(polar());
    let mut vertical_speed = Errors::default();
    let mut netto = Errors::default();
    let mut wind_speed = Errors::default();
    let mut wind_direction = Errors::default();

    let mut fix_extensions = Vec::new();
    let mut wind_extensions = Vec::new();
    let mut estimated_wind = None;

    for line in RECORDING.lines() {
        match Record::parse_line(line) {
            Ok(Record::I(definition)) => fix_extensions = definition.0.extensions,
            Ok(Record::J(definition)) => wind_extensions = definition.0.extensions,
            Ok(Record::B(fix)) => {
                let value = |mnemonic| extension(&fix, &fix_extensions, mnemonic);
                let sample = Sample {
                    track: Angle::from_degrees(value("TRT")),
                    ground_speed: hundredths_kmh(value("GSP")),
                    pressure_altitude: PressureAltitude::new(Length::from_meters(f64::from(
                        fix.pressure_alt,
                    ))),
                    // A zero GNSS altitude means the recorder had no fix.
                    gnss_altitude: (fix.gps_alt != 0).then(|| {
                        EllipsoidAltitude::new(Length::from_meters(f64::from(fix.gps_alt)))
                    }),
                    true_air_speed: hundredths_kmh(value("TAS")),
                    position_accuracy: Length::from_meters(value("FXA")),
                };

                let Some(state) = estimator.update(seconds(&fix.timestamp), &sample) else {
                    continue;
                };
                estimated_wind = state.wind;

                vertical_speed.add(
                    state.vertical_speed.as_meters_per_second(),
                    value("VAT") / 100.,
                );
                netto.add(state.netto.as_meters_per_second(), value("NET") / 100.);
            }
            Ok(Record::K(record)) => {
                let Some(wind) = estimated_wind else { continue };
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

    let mut report = String::new();
    writeln!(
        report,
        "quantity           unit        n    rms    mae   bias   corr"
    )
    .unwrap();
    writeln!(report, "vertical speed     m/s   {}", vertical_speed.row()).unwrap();
    writeln!(report, "netto              m/s   {}", netto.row()).unwrap();
    writeln!(report, "wind speed         m/s   {}", wind_speed.row()).unwrap();
    writeln!(report, "wind direction     deg   {}", wind_direction.row()).unwrap();
    insta::assert_snapshot!(report);
}

fn polar() -> GlidePolar {
    POLAR_STORE
        .iter()
        .find(|entry| entry.name == GLIDER_TYPE)
        .expect("the built-in store has the recording's glider type")
        .glide_polar()
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
/// Correlation needs both series, so [`add_difference`](Self::add_difference)
/// leaves it undefined: a compass direction has no meaningful correlation.
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
        let count = self.count as f64;
        let correlation = match &self.correlation {
            Some(c) => {
                let covariance = c.products / count - c.estimates * c.references / (count * count);
                let estimates = c.estimate_squares / count - (c.estimates / count).powi(2);
                let references = c.reference_squares / count - (c.references / count).powi(2);
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
