use super::super::*;
use super::support::*;
use crate::{
    ChangeTask, ContinueFlight, FlightBoundaryQuestion, NavigationTarget, RestoreRecording,
    RestoreTask, Sample, StartNewFlight, StoredSample, Task, TaskCommand, TaskPoint, TaskTime,
    Velocity,
};
use claims::{assert_none, assert_ok, assert_some, assert_some_eq};
use updraft_geo::LatLon;
use updraft_units::Speed;

/// 2026-01-01T12:00:00Z
const UTC: i64 = 1_767_268_800_000;
const HOURS: i64 = 60 * 60 * 1000;
const DAYS: i64 = 24 * HOURS;

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
    assert_none!(question(&core));
}

fn question_emissions(effects: &[Effect]) -> Vec<Option<FlightBoundaryQuestion>> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Emit(Topic::FlightBoundaryQuestion(question)) => Some(*question),
            _ => None,
        })
        .collect()
}

#[test]
fn fix_more_than_3_hours_after_the_last_sample_asks_the_pilot_and_continues_the_recording() {
    let mut core = Core::new(SettingsSnapshot::default());
    record(&mut core, UTC, 0);
    assert_eq!(starts(record(&mut core, UTC + 3 * HOURS, 1_000)), [false]);
    assert_none!(question(&core));

    let effects = core
        .apply(utc_fix(UTC + 6 * HOURS + 1, 50.0), at(2_000))
        .effects;

    let question = FlightBoundaryQuestion::FixUtcJump {
        milliseconds: 3 * HOURS + 1,
    };
    assert_eq!(question_emissions(&effects), [Some(question)]);
    assert_eq!(starts(effects), [false]);
}

#[test]
fn fix_more_than_30_seconds_before_the_last_sample_asks_the_pilot_and_is_dropped() {
    let mut core = Core::new(SettingsSnapshot::default());
    record(&mut core, UTC, 0);

    let effects = core.apply(utc_fix(UTC - 30_001, 50.0), at(1_000)).effects;

    let question = FlightBoundaryQuestion::FixUtcJump {
        milliseconds: -30_001,
    };
    assert_eq!(question_emissions(&effects), [Some(question)]);
    assert_eq!(starts(effects), [false; 0]);
}

#[test]
fn continue_flight_after_a_backward_jump_records_no_fix_before_the_last_sample() {
    let mut core = Core::new(SettingsSnapshot::default());
    record(&mut core, UTC, 0);
    record(&mut core, UTC - 3 * DAYS, 1_000);

    let effects = core.apply(ContinueFlight, at(2_000)).effects;

    assert_eq!(question_emissions(&effects), [None]);
    assert_eq!(record(&mut core, UTC - 3 * DAYS + 1_000, 3_000), []);
    assert_none!(question(&core));
    assert_eq!(starts(record(&mut core, UTC + 1_000, 4_000)), [false]);
}

#[test]
fn new_flight_starts_a_recording_at_the_next_fix() {
    let mut core = Core::new(SettingsSnapshot::default());
    record(&mut core, UTC, 0);
    record(&mut core, UTC - 3 * DAYS, 1_000);

    let effects = core.apply(StartNewFlight, at(2_000)).effects;

    assert_eq!(question_emissions(&effects), [None]);
    assert_eq!(starts(effects), [false; 0]);
    assert_eq!(
        starts(record(&mut core, UTC - 3 * DAYS + 1_000, 3_000)),
        [true]
    );
    assert_eq!(
        starts(record(&mut core, UTC - 3 * DAYS + 2_000, 4_000)),
        [false]
    );
}

#[test]
fn new_flight_resets_the_wind() {
    let mut core = Core::new(SettingsSnapshot::default());
    restore(&mut core, UTC, UTC + HOURS);
    assert_some!(assert_some!(instruments(&core).derived).wind);

    core.apply(StartNewFlight, at(0));

    assert_eq!(
        instruments(&core).derived.and_then(|derived| derived.wind),
        None
    );
}

/// Returns `true` for each recorded fix that starts a recording.
fn starts(effects: Vec<Effect>) -> Vec<bool> {
    recording_effects(effects)
        .iter()
        .map(|effect| matches!(effect, Effect::StartRecording(_)))
        .collect()
}

fn shell_utc(core: &mut Core, utc: i64, at_millis: u64) {
    let utc = UtcInstant::from_unix_milliseconds(utc);
    core.apply(UtcTick::new(utc), at(at_millis));
}

#[test]
fn fix_more_than_3_hours_of_shell_time_after_the_last_sample_starts_a_recording() {
    let mut core = Core::new(SettingsSnapshot::default());
    shell_utc(&mut core, UTC, 0);
    record(&mut core, UTC, 0);
    shell_utc(&mut core, UTC + 3 * HOURS, 1_000);
    assert_eq!(starts(record(&mut core, UTC + 1_000, 1_000)), [false]);
    shell_utc(&mut core, UTC + 6 * HOURS + 1, 2_000);
    assert_eq!(starts(record(&mut core, UTC + 2_000, 2_000)), [true]);
}

#[test]
fn new_recording_after_more_than_3_hours_of_shell_time_resets_the_wind() {
    let mut core = Core::new(SettingsSnapshot::default());
    restore(&mut core, UTC, UTC + HOURS);
    shell_utc(&mut core, UTC + 3 * HOURS + 1, 0);

    let effects = core.apply(utc_fix(UTC + 1_000, 50.0), at(0)).effects;

    let instruments = single_instruments_emission(&effects);
    assert_none!(
        instruments
            .derived
            .as_ref()
            .and_then(|derived| derived.wind)
    );
    assert_eq!(starts(effects), [true]);
}

#[test]
fn new_recording_after_more_than_3_hours_of_shell_time_closes_the_question() {
    let mut core = Core::new(SettingsSnapshot::default());
    shell_utc(&mut core, UTC, 0);
    record(&mut core, UTC, 0);
    record(&mut core, UTC - 3 * DAYS, 1_000);
    assert_some!(question(&core));
    shell_utc(&mut core, UTC + 3 * HOURS + 2_001, 2_000);

    let effects = core.apply(utc_fix(UTC + 2_000, 50.0), at(2_000)).effects;

    assert_eq!(question_emissions(&effects), [None]);
    assert_eq!(starts(effects), [true]);
}

fn waypoint(name: &str, latitude_degrees: f64) -> NavigationTarget {
    NavigationTarget::Waypoint {
        name: name.into(),
        latitude_degrees,
        longitude_degrees: 6.0,
        elevation_meters: 0.,
    }
}

#[test]
fn new_flight_resets_task_progress() {
    let mut core = Core::new(SettingsSnapshot::default());
    for (name, latitude_degrees) in [("Start", 50.0), ("Finish", 51.0)] {
        let target = waypoint(name, latitude_degrees);
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
    assert_some_eq!(started.target, 1);
    assert_some_eq!(
        started.progress.start,
        TaskTime {
            unix_milliseconds: UTC + 450
        }
    );

    core.apply(StartNewFlight, at(2_000));
    core.apply(utc_fix(UTC + 2_000, 50.0), at(2_000));
    insta::assert_debug_snapshot!(core.task.snapshot());
}

fn restored_sample(utc: i64, latitude_degrees: f64) -> StoredSample {
    let sample = Sample {
        utc: UtcInstant::from_unix_milliseconds(utc),
        position: LatLon::from_degrees(latitude_degrees, 6.0),
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
    StoredSample {
        sample,
        terrain_elevation_meters: None,
    }
}

/// Restores a recording that the shell stored at the UTC of its last sample.
fn restore(core: &mut Core, last_utc: i64, utc: i64) -> Vec<Effect> {
    restore_stored(core, last_utc, last_utc, utc)
}

fn restore_stored(core: &mut Core, last_utc: i64, stored_utc: i64, utc: i64) -> Vec<Effect> {
    let samples = vec![
        restored_sample(last_utc - 1_000, 50.0),
        restored_sample(last_utc, 50.0),
    ];
    let input = RestoreRecording {
        samples,
        stored_utc: Some(UtcInstant::from_unix_milliseconds(stored_utc)),
        utc: UtcInstant::from_unix_milliseconds(utc),
    };
    core.apply(input, at(0)).effects
}

#[test]
fn restore_discards_a_recording_that_the_shell_stored_more_than_3_hours_ago() {
    let mut core = Core::new(SettingsSnapshot::default());
    let last_utc = UTC + 3 * HOURS;
    let mut effects = restore_stored(&mut core, last_utc, UTC, UTC + 3 * HOURS + 1);
    effects.extend(record(&mut core, UTC + 1_000, 1_000));
    insta::assert_debug_snapshot!(effects);
}

#[test]
fn restore_continues_a_recording_with_an_old_fix_utc_that_the_shell_stored_recently() {
    let mut core = Core::new(SettingsSnapshot::default());
    let last_utc = UTC - 30 * 24 * HOURS;

    assert_eq!(
        restore_stored(&mut core, last_utc, UTC, UTC + 3 * HOURS),
        []
    );

    assert_eq!(record(&mut core, last_utc, 0), []);
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

/// Restores a recording that exits the start and enters `TP1`.
fn restore_flight(core: &mut Core) {
    let samples = [(0, 50.0), (1_000, 50.01), (60_000, 50.095), (61_000, 50.1)]
        .map(|(offset, latitude_degrees)| restored_sample(UTC + offset, latitude_degrees));
    let input = RestoreRecording {
        samples: samples.into(),
        stored_utc: Some(UtcInstant::from_unix_milliseconds(UTC + 61_000)),
        utc: UtcInstant::from_unix_milliseconds(UTC + HOURS),
    };
    core.apply(input, at(0));
}

fn restore_route(core: &mut Core, route: &[(&str, f64)]) {
    let points = route
        .iter()
        .zip(0..)
        .map(|(&(name, latitude_degrees), id)| TaskPoint {
            id,
            target: waypoint(name, latitude_degrees),
        })
        .collect::<Vec<_>>();
    let next_id = points.len() as u32;
    let task = Task { points, next_id };
    assert_ok!(core.apply(RestoreTask(task), at(0)).response);
}

#[test]
fn restore_derives_task_progress_from_the_recording() {
    let mut core = Core::new(SettingsSnapshot::default());
    restore_route(
        &mut core,
        &[("Start", 50.0), ("TP1", 50.1), ("Finish", 50.2)],
    );
    restore_flight(&mut core);
    let task = core.task.snapshot();
    insta::assert_debug_snapshot!((task.target, task.progress));
}

/// Restores the recording before the route, in the order of the shell
/// startup.
#[test]
fn route_change_after_restore_derives_task_progress_from_restored_and_live_samples() {
    let mut core = Core::new(SettingsSnapshot::default());
    restore_flight(&mut core);
    let route = [
        ("Start", 50.0),
        ("TP1", 50.1),
        ("TP2", 50.2),
        ("Finish", 50.3),
    ];
    restore_route(&mut core, &route);
    core.apply(utc_fix(UTC + 120_000, 50.195), at(1_000));
    core.apply(utc_fix(UTC + 121_000, 50.2), at(2_000));
    let remove = ChangeTask(TaskCommand::Remove { id: 1 });
    assert_ok!(core.apply(remove, at(3_000)).response);
    let task = core.task.snapshot();
    insta::assert_debug_snapshot!((task.target, task.progress));
}
