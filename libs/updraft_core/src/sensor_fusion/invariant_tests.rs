//! Checks physical estimator properties against a recorded flight.

use claims::{assert_ge, assert_le};
use igc::records::{Extendable, Extension, Record};
use std::time::Duration;
use updraft_geo::LatLon;
use updraft_polar::{GlidePolar, POLAR_STORE};
use updraft_units::{Angle, EllipsoidAltitude, Length, PressureAltitude, Speed};

use super::estimator::{Estimate, Estimator, Fix};
use super::sample::SampleAcceptance::Accepted;

const RECORDING: &str = include_str!("../../../../testdata/weglide_1141558.igc");
const GLIDER_TYPE: &str = "JS-3-18m";

/// Bank angle above which a sample counts as circling, in degrees.
const CIRCLING_BANK: f64 = 20.;

fn replay() -> Vec<Estimate> {
    let mut estimator = Estimator::new();
    estimator.set_polar(polar());
    let mut extensions = Vec::new();
    let mut samples = Vec::new();

    for line in RECORDING.lines() {
        match Record::parse_line(line) {
            Ok(Record::I(definition)) => extensions = definition.0.extensions,
            Ok(Record::B(record)) => {
                let value = |mnemonic| extension(&record, &extensions, mnemonic);
                let time = seconds(&record.timestamp);
                let air_speed = hundredths_kmh(value("TAS"));
                assert_eq!(estimator.air_speed(time, air_speed), Accepted);
                estimator.position(LatLon::from_degrees(
                    f64::from(record.pos.lat),
                    f64::from(record.pos.lon),
                ));
                let _ = estimator.fix(
                    time,
                    &Fix {
                        track: Angle::from_degrees(value("TRT")),
                        ground_speed: hundredths_kmh(value("GSP")),
                    },
                );
                if record.gps_alt != 0 {
                    let _ = estimator.gnss_altitude(
                        time,
                        EllipsoidAltitude::new(Length::from_meters(f64::from(record.gps_alt))),
                    );
                }
                let _ = estimator.pressure_altitude(
                    time,
                    PressureAltitude::new(Length::from_meters(f64::from(record.pressure_alt))),
                );
                samples.push(estimator.estimate());
            }
            _ => {}
        }
    }
    samples
}

/// The sink rate the estimate applied, which is the netto with the
/// vertical speed taken back out.
fn applied_sink(state: &Estimate) -> Option<f64> {
    Some((state.netto? - state.vario?).as_meters_per_second())
}

#[test]
fn applied_sink_rate_is_never_negative() {
    let worst = replay()
        .iter()
        .filter_map(applied_sink)
        .fold(f64::INFINITY, f64::min);

    // A glider cannot sink slower than its own best sink rate, whatever
    // the air is doing: altitude and bank angle only ever raise it. That
    // bound is 0.446 m/s here and the flight comes within 0.09 m/s of
    // it, so it constrains the reading rather than merely asserting that
    // the glider does not rise through the air it flies in.
    //
    // Rising through it is what the *recorded* netto of three flights in
    // this investigation does, which is why this is measured and not
    // compared.
    let best = polar().min_sink_rate().as_meters_per_second();
    assert_ge!(worst, best);
}

#[test]
fn applied_sink_rate_does_not_depend_on_turn_direction() {
    const BAND: f64 = 2.;
    const MIN_SAMPLES: usize = 100;
    const MIN_BANDS: usize = 5;

    let samples = replay();
    let mean = |right: bool, band: i32| {
        let (sum, count) = samples
            .iter()
            .filter(|state| {
                state.bank_angle.is_some_and(|angle| {
                    let degrees = angle.as_degrees();
                    degrees.abs() >= CIRCLING_BANK
                        && (degrees > 0.) == right
                        && (degrees.abs() / BAND) as i32 == band
                })
            })
            .filter_map(applied_sink)
            .fold((0., 0), |(sum, count), value| (sum + value, count + 1));
        (sum / count as f64, count)
    };

    // The load factor comes from the cosine of the bank angle, which is
    // even, so the two directions have to agree at the same bank angle.
    //
    // At the *same* bank angle. This pilot holds 42.6° turning right and
    // 41.4° turning left, and the load factor turns that 1.2° into 3% of
    // sink rate, so comparing the two directions in one lump measures
    // the pilot instead of the estimate. Inside a band what is left is
    // that the glider flew the two at different speeds, half a metre per
    // second apart here, and that measures 0.017 m/s.
    //
    // The instrument that recorded this flight disagrees with itself by
    // 2.64 m/s, which is why this is measured and not compared.
    let mut compared = 0;
    for band in 0..(90. / BAND) as i32 {
        let (right, right_count) = mean(true, band);
        let (left, left_count) = mean(false, band);
        if right_count < MIN_SAMPLES || left_count < MIN_SAMPLES {
            continue;
        }
        compared += 1;
        assert_le!((right - left).abs(), 0.03);
    }
    assert_ge!(compared, MIN_BANDS);
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

fn hundredths_kmh(value: f64) -> Speed {
    Speed::from_kilometers_per_hour(value / 100.)
}

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
