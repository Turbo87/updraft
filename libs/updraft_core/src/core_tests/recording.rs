use super::super::*;
use super::support::*;
use crate::{
    ChangeTask, NavigationTarget, RestoreRecording, Sample, TaskCommand, TaskTime, Velocity,
};
use claims::{assert_ok, assert_some, assert_some_eq};
use updraft_geo::LatLon;
use updraft_units::Speed;

/// 2026-01-01T12:00:00Z
const UTC: i64 = 1_767_268_800_000;
const HOURS: i64 = 60 * 60 * 1000;

fn utc_fix(utc: i64, latitude_degrees: f64) -> InternalGps {
    InternalGps::new(Fix {
        fix_time: Some(UtcInstant::from_unix_milliseconds(utc)),
        ..fix(latitude_degrees, 6.0)
    })
}

fn recording_effects(effects: Vec<Effect>) -> Vec<Effect> {
    effects
        .into_iter()
        .filter(|effect| matches!(effect, Effect::StartRecording(_) | Effect::RecordSample(_)))
        .collect()
}

fn record(core: &mut Core, utc: i64, at_millis: u64) -> Vec<Effect> {
    recording_effects(core.apply(utc_fix(utc, 50.0), at(at_millis)).effects)
}

#[test]
fn first_fix_starts_a_recording_and_later_fixes_append() {
    let mut core = Core::new(SettingsSnapshot::default());
    let mut effects = record(&mut core, UTC, 0);
    effects.extend(record(&mut core, UTC + 1_000, 1_000));
    insta::assert_debug_snapshot!(effects);
}

#[test]
fn fix_without_utc_is_not_recorded() {
    let mut core = Core::new(SettingsSnapshot::default());
    let effects = core.apply(InternalGps::new(fix(50.0, 6.0)), at(0)).effects;
    assert_eq!(recording_effects(effects), []);
}

#[test]
fn fix_at_or_up_to_30_seconds_before_the_last_sample_is_dropped() {
    let mut core = Core::new(SettingsSnapshot::default());
    record(&mut core, UTC, 0);
    assert_eq!(record(&mut core, UTC, 1_000), []);
    assert_eq!(record(&mut core, UTC - 30_000, 2_000), []);
    insta::assert_debug_snapshot!(record(&mut core, UTC - 30_001, 3_000));
}

#[test]
fn fix_more_than_3_hours_after_the_last_sample_starts_a_recording() {
    let mut core = Core::new(SettingsSnapshot::default());
    record(&mut core, UTC, 0);
    let mut effects = record(&mut core, UTC + 3 * HOURS, 1_000);
    effects.extend(record(&mut core, UTC + 6 * HOURS + 1, 2_000));
    insta::assert_debug_snapshot!(effects);
}

#[test]
fn new_recording_resets_task_progress() {
    let mut core = Core::new(SettingsSnapshot::default());
    for (name, latitude_degrees) in [("Start", 50.0), ("Finish", 51.0)] {
        let target = NavigationTarget::Waypoint {
            name: name.into(),
            latitude_degrees,
            longitude_degrees: 6.0,
            elevation_meters: 0.,
        };
        assert_ok!(
            core.apply(ChangeTask(TaskCommand::Add { target }), at(0))
                .response
        );
    }
    assert_ok!(
        core.apply(ChangeTask(TaskCommand::Select { id: 0 }), at(0))
            .response
    );
    core.apply(utc_fix(UTC, 50.0), at(0));
    core.apply(utc_fix(UTC + 1_000, 50.01), at(1_000));
    let started = core.task.snapshot();
    assert_some_eq!(started.current, 1);
    assert_some_eq!(
        started.start,
        TaskTime {
            unix_milliseconds: UTC + 450
        }
    );

    core.apply(utc_fix(UTC + 4 * HOURS, 50.0), at(2_000));
    insta::assert_debug_snapshot!(core.task.snapshot());
}

fn restored_sample(utc: i64) -> Sample {
    Sample {
        utc: UtcInstant::from_unix_milliseconds(utc),
        position: LatLon::from_degrees(50.0, 6.0),
        altitude_msl: None,
        vario: None,
        netto: None,
        relative_vario: None,
        wind: Some(Velocity {
            east: Speed::from_meters_per_second(-3.0),
            north: Speed::from_meters_per_second(4.0),
        }),
    }
}

fn restore(core: &mut Core, last_utc: i64, utc: i64) -> Vec<Effect> {
    let samples = vec![restored_sample(last_utc - 1_000), restored_sample(last_utc)];
    let input = RestoreRecording {
        samples,
        utc: UtcInstant::from_unix_milliseconds(utc),
    };
    core.apply(input, at(0)).effects
}

#[test]
fn restore_discards_a_recording_that_ended_more_than_3_hours_ago() {
    let mut core = Core::new(SettingsSnapshot::default());
    let mut effects = restore(&mut core, UTC, UTC + 3 * HOURS + 1);
    effects.extend(record(&mut core, UTC + 1_000, 1_000));
    insta::assert_debug_snapshot!(effects);
}

#[test]
fn restore_seeds_the_wind_from_the_last_sample() {
    let mut core = Core::new(SettingsSnapshot::default());
    assert_eq!(restore(&mut core, UTC, UTC + 3 * HOURS), []);
    let derived = assert_some!(instruments(&core).derived);
    insta::assert_debug_snapshot!(derived.wind);
}

/// Rounds the sample wind to 1 µm/s. The wind passes through `atan2()`,
/// `sin()`, and `cos()`, and their last bit differs between platforms.
fn round_wind(mut effects: Vec<Effect>) -> Vec<Effect> {
    let round = |speed: Speed| {
        Speed::from_meters_per_second((speed.as_meters_per_second() * 1e6).round() / 1e6)
    };
    for effect in &mut effects {
        if let Effect::StartRecording(sample) | Effect::RecordSample(sample) = effect
            && let Some(wind) = &mut sample.wind
        {
            wind.east = round(wind.east);
            wind.north = round(wind.north);
        }
    }
    effects
}

#[test]
fn live_fix_after_restore_appends_to_the_recording() {
    let mut core = Core::new(SettingsSnapshot::default());
    restore(&mut core, UTC, UTC + HOURS);
    assert_eq!(record(&mut core, UTC, 0), []);
    insta::assert_debug_snapshot!(round_wind(record(&mut core, UTC + 1_000, 1_000)));
}
