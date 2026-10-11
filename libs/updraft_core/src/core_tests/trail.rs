use super::super::*;
use super::support::*;
use crate::{RestoreRecording, Sample, StoredSample, TerrainElevation, Trail};
use claims::{assert_none, assert_some};
use updraft_geo::LatLon;
use updraft_units::{EllipsoidAltitude, Length, MslAltitude, Speed};

/// 2026-01-01T12:00:00Z
const UTC: i64 = 1_767_268_800_000;
const HOURS: i64 = 60 * 60 * 1000;

fn utc_fix(utc: i64, latitude_degrees: f64) -> InternalGps {
    InternalGps::new(Fix {
        fix_time: Some(UtcInstant::from_unix_milliseconds(utc)),
        ..fix(latitude_degrees, 6.0)
    })
}

/// The geoid undulation at 50°N 6°E is 48 m, so this fix has an MSL
/// altitude of `ellipsoid_meters - 48`.
fn altitude_fix(utc: i64, ellipsoid_meters: f64) -> InternalGps {
    InternalGps::new(Fix {
        fix_time: Some(UtcInstant::from_unix_milliseconds(utc)),
        altitude_ellipsoid: Some(EllipsoidAltitude::new(Length::from_meters(
            ellipsoid_meters,
        ))),
        ..fix(50.0, 6.0)
    })
}

fn maxima(trail: Option<Trail>) -> (Option<f64>, Option<f64>) {
    let trail = assert_some!(trail);
    (trail.max_altitude_msl_meters, trail.max_altitude_agl_meters)
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

#[test]
fn trail_keeps_the_running_maxima_of_the_recording() {
    let mut core = Core::new(SettingsSnapshot::default());
    let mut topics = trail_topics(core.apply(altitude_fix(UTC, 1_048.0), at(0)).effects);
    let position = gps_instruments(&core).position;
    core.apply(
        TerrainElevation {
            position,
            meters: Some(300.0),
        },
        at(1),
    );
    for (offset, ellipsoid_meters) in [(1_000, 1_548.0), (2_000, 1_248.0), (4 * HOURS, 548.0)] {
        let input = altitude_fix(UTC + offset, ellipsoid_meters);
        topics.extend(trail_topics(core.apply(input, at(offset as u64)).effects));
    }
    let maxima: Vec<_> = topics.into_iter().map(maxima).collect();
    assert_eq!(
        maxima,
        [
            (Some(1_000.0), Some(1_000.0)),
            (Some(1_500.0), Some(1_200.0)),
            (Some(1_500.0), Some(1_200.0)),
            (Some(500.0), Some(200.0)),
        ]
    );
}

fn restored_sample(utc: i64) -> Sample {
    Sample {
        utc: UtcInstant::from_unix_milliseconds(utc),
        position: LatLon::from_degrees(50.0, 6.0),
        altitude_msl: Some(MslAltitude::new(Length::from_meters(1_000.0))),
        vario: Some(Speed::from_meters_per_second(1.5)),
        vertical_speed: Some(Speed::from_meters_per_second(1.25)),
        netto: Some(Speed::from_meters_per_second(2.5)),
        relative_vario: Some(Speed::from_meters_per_second(3.5)),
        estimated_netto: Some(Speed::from_meters_per_second(2.25)),
        estimated_relative_vario: Some(Speed::from_meters_per_second(3.25)),
        wind: None,
    }
}

fn stored_sample(utc: i64, altitude_msl: Option<f64>, terrain: Option<f64>) -> StoredSample {
    StoredSample {
        sample: Sample {
            altitude_msl: altitude_msl.map(|meters| MslAltitude::new(Length::from_meters(meters))),
            ..restored_sample(utc)
        },
        terrain_elevation_meters: terrain,
    }
}

fn restore_samples(core: &mut Core, samples: Vec<StoredSample>, utc: i64) -> Vec<Effect> {
    let last_utc = samples.last().map(|stored| stored.sample.utc);
    let input = RestoreRecording {
        samples,
        stored_utc: last_utc,
        utc: UtcInstant::from_unix_milliseconds(utc),
    };
    core.apply(input, at(0)).effects
}

fn restore(core: &mut Core, last_utc: i64, utc: i64) -> Vec<Effect> {
    let samples = vec![
        stored_sample(last_utc - 1_000, Some(1_000.0), Some(100.0)),
        stored_sample(last_utc, Some(1_000.0), Some(100.0)),
    ];
    restore_samples(core, samples, utc)
}

#[test]
fn trail_continues_the_restored_recording() {
    let mut core = Core::new(SettingsSnapshot::default());
    assert_none!(current_trail(&core));
    restore(&mut core, UTC, UTC + HOURS);
    insta::assert_debug_snapshot!(current_trail(&core));
}

#[test]
fn trail_calculates_the_maxima_again_at_restore() {
    let mut core = Core::new(SettingsSnapshot::default());
    let samples = vec![
        stored_sample(UTC - 3_000, Some(1_000.0), Some(0.0)),
        stored_sample(UTC - 2_000, Some(1_500.0), Some(800.0)),
        stored_sample(UTC - 1_000, None, Some(0.0)),
        stored_sample(UTC, Some(1_200.0), None),
    ];
    restore_samples(&mut core, samples, UTC);
    assert_eq!(maxima(current_trail(&core)), (Some(1_500.0), Some(1_200.0)));

    let effects = core
        .apply(altitude_fix(UTC + 1_000, 1_648.0), at(1))
        .effects;
    let topics: Vec<_> = trail_topics(effects).into_iter().map(maxima).collect();
    assert_eq!(topics, [(Some(1_600.0), Some(1_600.0))]);
}

#[test]
fn trail_is_none_after_a_discard_at_restore() {
    let mut core = Core::new(SettingsSnapshot::default());
    restore(&mut core, UTC, UTC + 24 * HOURS);
    assert_none!(current_trail(&core));
}
