use super::super::*;
use super::support::*;
use crate::{RestoreRecording, Sample, TerrainElevation, Trail};
use claims::assert_none;
use updraft_geo::LatLon;
use updraft_units::{Length, MslAltitude, Speed};

/// 2026-01-01T12:00:00Z
const UTC: i64 = 1_767_268_800_000;
const HOURS: i64 = 60 * 60 * 1000;

fn utc_fix(utc: i64, latitude_degrees: f64) -> InternalGps {
    InternalGps::new(Fix {
        fix_time: Some(UtcInstant::from_unix_milliseconds(utc)),
        ..fix(latitude_degrees, 6.0)
    })
}

fn trail_topics(effects: Vec<Effect>) -> Vec<Option<Trail>> {
    effects
        .into_iter()
        .filter_map(|effect| match effect {
            Effect::Emit(Topic::Trail(trail)) => Some(trail),
            _ => None,
        })
        .collect()
}

fn current_trail(core: &Core) -> Option<Trail> {
    core.topics()
        .into_iter()
        .find_map(|topic| match topic {
            Topic::Trail(trail) => Some(trail),
            _ => None,
        })
        .expect("the trail topic should be published")
}

#[test]
fn trail_publishes_each_recorded_sample() {
    let mut core = Core::new(SettingsSnapshot::default());
    let mut topics = trail_topics(core.apply(utc_fix(UTC, 50.0), at(0)).effects);
    let position = gps_instruments(&core).position;
    core.apply(
        TerrainElevation {
            position,
            meters: Some(100.0),
        },
        at(1),
    );
    topics.extend(trail_topics(
        core.apply(utc_fix(UTC + 1_000, 50.0001), at(1_000)).effects,
    ));
    insta::assert_debug_snapshot!(topics);
    assert_eq!(current_trail(&core), topics[1]);
}

fn restored_sample(utc: i64) -> Sample {
    Sample {
        utc: UtcInstant::from_unix_milliseconds(utc),
        position: LatLon::from_degrees(50.0, 6.0),
        altitude_msl: Some(MslAltitude::new(Length::from_meters(1_000.0))),
        vario: Some(Speed::from_meters_per_second(1.5)),
        netto: Some(Speed::from_meters_per_second(2.5)),
        relative_vario: Some(Speed::from_meters_per_second(3.5)),
        wind: None,
    }
}

fn restore(core: &mut Core, last_utc: i64, utc: i64) -> Vec<Effect> {
    let input = RestoreRecording {
        samples: vec![restored_sample(last_utc - 1_000), restored_sample(last_utc)],
        stored_utc: Some(UtcInstant::from_unix_milliseconds(last_utc)),
        utc: UtcInstant::from_unix_milliseconds(utc),
    };
    core.apply(input, at(0)).effects
}

#[test]
fn trail_continues_the_restored_recording() {
    let mut core = Core::new(SettingsSnapshot::default());
    assert_none!(current_trail(&core));
    restore(&mut core, UTC, UTC + HOURS);
    insta::assert_debug_snapshot!(current_trail(&core));
}

#[test]
fn trail_is_none_after_a_discard_at_restore() {
    let mut core = Core::new(SettingsSnapshot::default());
    restore(&mut core, UTC, UTC + 24 * HOURS);
    assert_none!(current_trail(&core));
}
