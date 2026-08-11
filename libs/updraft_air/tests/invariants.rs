//! Checks the estimate against physics rather than against an instrument.
//!
//! Every figure in `recorded_flight.rs` is a difference against values an
//! LXNAV instrument recorded, and that reference is not ground truth: its
//! netto changes with the direction of turn, and three of the recordings
//! behind this investigation hold a negative sink rate. A difference
//! against it therefore cannot say whether the estimate is right.
//!
//! These tests do not need a reference. They state properties the answer
//! has to hold whatever the air was doing, so they stay meaningful on any
//! recording, including one with no instrument values at all.

use igc::records::{Extendable, Extension, Record};
use std::time::Duration;
use updraft_air::{AirState, AirStateEstimator, Fix};
use updraft_geo::LatLon;
use updraft_polar::{GlidePolar, POLAR_STORE};
use updraft_units::{Angle, EllipsoidAltitude, Length, PressureAltitude, Speed};

const RECORDING: &str = include_str!("../../../testdata/weglide_1141558.igc");
const GLIDER_TYPE: &str = "JS-3-18m";

/// Bank angle above which a sample counts as circling, in degrees.
const CIRCLING_BANK: f64 = 20.;

/// One second of the recording, with the state it produced.
struct Sample {
    time: Duration,
    state: AirState,
}

fn replay() -> Vec<Sample> {
    let mut estimator = AirStateEstimator::new().with_polar(polar());
    let mut extensions = Vec::new();
    let mut samples = Vec::new();

    for line in RECORDING.lines() {
        match Record::parse_line(line) {
            Ok(Record::I(definition)) => extensions = definition.0.extensions,
            Ok(Record::B(record)) => {
                let value = |mnemonic| extension(&record, &extensions, mnemonic);
                let time = seconds(&record.timestamp);
                estimator.air_speed(time, hundredths_kmh(value("TAS")));
                estimator.fix(
                    time,
                    &Fix {
                        position: LatLon::from_degrees(
                            f64::from(record.pos.lat),
                            f64::from(record.pos.lon),
                        ),
                        track: Angle::from_degrees(value("TRT")),
                        ground_speed: hundredths_kmh(value("GSP")),
                        position_accuracy: Length::from_meters(value("FXA")),
                    },
                );
                if record.gps_alt != 0 {
                    estimator.gnss_altitude(
                        time,
                        EllipsoidAltitude::new(Length::from_meters(f64::from(record.gps_alt))),
                    );
                }
                estimator.pressure_altitude(
                    time,
                    PressureAltitude::new(Length::from_meters(f64::from(record.pressure_alt))),
                );
                if let Some(state) = estimator.state() {
                    samples.push(Sample { time, state });
                }
            }
            _ => {}
        }
    }
    samples
}

/// The sink rate the estimate applied, which is the netto with the
/// vertical speed taken back out.
fn applied_sink(state: &AirState) -> Option<f64> {
    let netto = state.netto?.as_meters_per_second();
    Some(netto - state.vertical_speed.as_meters_per_second())
}

#[test]
fn the_applied_sink_rate_is_never_negative() {
    let samples = replay();
    let worst = samples
        .iter()
        .filter_map(|sample| applied_sink(&sample.state))
        .fold(f64::INFINITY, f64::min);

    // A glider cannot sink slower than its own best sink rate, whatever
    // the air is doing: height and bank angle only ever raise it. That
    // bound is 0.446 m/s here and the flight comes within 0.09 m/s of
    // it, so it constrains the reading rather than merely asserting that
    // the glider does not rise through the air it flies in.
    //
    // Rising through it is what the *recorded* netto of three flights in
    // this investigation does, which is why this is measured and not
    // compared.
    let best = polar().min_sink_rate().as_meters_per_second();
    assert!(
        worst >= best,
        "sink rate reached {worst:.3} m/s, below the polar's best of {best:.3} m/s"
    );
}

#[test]
fn the_applied_sink_rate_does_not_depend_on_the_direction_of_turn() {
    /// Width of a bank-angle band, in degrees.
    const BAND: f64 = 2.;
    /// Samples a band needs in each direction before it is compared.
    const MIN_SAMPLES: usize = 100;
    /// Bands that have to hold enough samples to compare.
    const MIN_BANDS: usize = 5;

    let samples = replay();
    let mean = |right: bool, band: i32| {
        let values: Vec<f64> = samples
            .iter()
            .filter(|sample| {
                sample.state.bank_angle.is_some_and(|bank| {
                    let degrees = bank.as_degrees();
                    degrees.abs() >= CIRCLING_BANK
                        && (degrees > 0.) == right
                        && (degrees.abs() / BAND) as i32 == band
                })
            })
            .filter_map(|sample| applied_sink(&sample.state))
            .collect();
        (
            values.iter().sum::<f64>() / values.len() as f64,
            values.len(),
        )
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
        assert!(
            (right - left).abs() <= 0.03,
            "at {:.0}–{:.0}° of bank, right turns {right:.3} m/s against left turns {left:.3} m/s",
            f64::from(band) * BAND,
            f64::from(band + 1) * BAND,
        );
    }
    assert!(compared >= MIN_BANDS, "only {compared} bands were compared");
}

#[test]
fn the_rate_of_climb_integrates_to_the_height_it_reports() {
    let samples = replay();
    let mut integrated = 0.;
    let mut start = None;
    let mut last = None;
    let mut previous = None;

    for sample in &samples {
        // Both sides have to begin together, so nothing counts until the
        // altitude has a sea-level reference to be measured against.
        let Some(altitude) = sample.state.altitude else {
            continue;
        };
        let altitude = altitude.into_inner().as_meters();
        let start = *start.get_or_insert(altitude);

        if let Some(previous) = previous {
            let interval = sample.time.saturating_sub(previous).as_secs_f64();
            integrated += sample.state.rate_of_climb.as_meters_per_second() * interval;
        }
        previous = Some(sample.time);
        last = Some(integrated - (altitude - start));
    }

    // The smoothing delays the reading but does not change its area, so
    // the two have to close over five hours and 1900 m of altitude. A
    // restart of the filter would lose the height it was reporting, and
    // a step folded in twice would gain it, and neither comes back.
    //
    // Only the closing figure is an invariant. Along the way the reading
    // lags the air by about four seconds, so a launch at 8 m/s legally
    // puts the integral 30 m behind and then returns it.
    let closing = last.expect("the recording reports an altitude");
    assert!(
        closing.abs() <= 5.,
        "integral closed {closing:.1} m from the height"
    );
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
