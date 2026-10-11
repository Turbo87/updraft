use super::super::*;
use super::support::*;
use crate::{RestoreRecording, Sample, StoredSample, Velocity};
use claims::assert_some;
use updraft_geo::LatLon;
use updraft_units::Speed;

/// 2026-01-01T12:00:00Z
const UTC: i64 = 1_767_268_800_000;

/// A core with a wind from a restored recording.
fn core_with_wind() -> Core {
    let mut core = Core::new(SettingsSnapshot::default());
    let sample = Sample {
        utc: UtcInstant::from_unix_milliseconds(UTC),
        position: LatLon::from_degrees(50.0, 6.0),
        altitude_msl: None,
        vario: None,
        vertical_speed: None,
        netto: None,
        relative_vario: None,
        estimated_netto: None,
        estimated_relative_vario: None,
        wind: Some(Velocity {
            east: Speed::from_meters_per_second(-3.0),
            north: Speed::from_meters_per_second(4.0),
        }),
    };
    let input = RestoreRecording {
        samples: vec![StoredSample {
            sample,
            terrain_elevation_meters: None,
        }],
        stored_utc: Some(sample.utc),
        utc: sample.utc,
    };
    core.apply(input, at(0));
    core
}

/// Applies one internal fix per second after the restored sample.
fn fly(core: &mut Core, seconds: std::ops::Range<u64>, latitude_degrees: f64) {
    for second in seconds {
        let utc = UtcInstant::from_unix_milliseconds(UTC + second as i64 * 1_000);
        let fix = InternalGps::new(Fix {
            fix_time: Some(utc),
            ..fix(latitude_degrees, 6.0)
        });
        core.apply(fix, at(second * 1_000));
    }
}

fn wind(core: &Core) -> Option<crate::topic::DerivedWindInstruments> {
    instruments(core).derived.and_then(|derived| derived.wind)
}

fn wind_state(core: &Core) -> &'static str {
    match wind(core) {
        Some(wind) if wind.stale => "stale",
        Some(_) => "current",
        None => "unavailable",
    }
}

#[test]
fn only_a_confirmed_position_jump_resets_the_fusion() {
    let mut core = core_with_wind();
    let track = [50.0, 50.0, 51.0, 50.0, 50.0, 51.0, 51.0, 51.0, 51.0];
    let log = track
        .into_iter()
        .zip(1..)
        .map(|(latitude, second)| {
            fly(&mut core, second..second + 1, latitude);
            format!("{second}: {latitude} {}", wind_state(&core))
        })
        .collect::<Vec<_>>();

    insta::assert_snapshot!(log.join("\n"));
}

#[test]
fn position_change_without_an_advancing_fix_utc_keeps_the_wind() {
    let mut core = core_with_wind();
    fly(&mut core, 1..3, 50.0);
    for second in 3..7 {
        let fix = InternalGps::new(Fix {
            fix_time: Some(UtcInstant::from_unix_milliseconds(UTC + 2_000)),
            ..fix(51.0, 6.0)
        });
        core.apply(fix, at(second * 1_000));
    }

    assert!(!assert_some!(wind(&core)).stale);
}
