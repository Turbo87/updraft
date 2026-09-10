use super::super::*;
use super::support::*;
use crate::ReplaceFlarmnetDatabase;
use crate::connection::{ConnectionSpec, ConnectionState};
use crate::settings::SettingsSnapshot;
use approx::assert_abs_diff_eq;
use claims::{assert_none, assert_ok, assert_some, assert_some_eq};
use std::assert_matches;
use updraft_flarmnet::FlarmnetDatabase;

#[test]
fn traffic_prefers_the_sending_devices_ownship_references() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4354)),
        ],
    });
    let first_device_id = device_id(&core, 0);
    let second_device_id = device_id(&core, 1);
    core.apply(Bytes::new(first_device_id, RMC), at(0));
    core.apply(Bytes::new(first_device_id, GGA), at(1));
    core.apply(Bytes::new(second_device_id, RMC_SECOND_DEVICE), at(2));
    core.apply(Bytes::new(second_device_id, GGA_SECOND_DEVICE), at(3));

    let effects = core
        .apply(Bytes::new(first_device_id, PFLAA_A), at(4))
        .effects;
    let delta = traffic_delta(&effects);
    let [target] = delta.upserts.as_slice() else {
        panic!("one accepted observation should produce one upsert");
    };

    assert_abs_diff_eq!(target.position.latitude_degrees, 50.832, epsilon = 1e-3);
    assert_abs_diff_eq!(target.position.longitude_degrees, 6.189, epsilon = 1e-3);
    assert_some_eq!(target.altitude_msl_meters, 250.0);
}

#[test]
fn traffic_falls_back_to_displayed_ownship_references() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4354)),
        ],
    });
    let first_device_id = device_id(&core, 0);
    let second_device_id = device_id(&core, 1);
    core.apply(Bytes::new(second_device_id, RMC_SECOND_DEVICE), at(0));
    core.apply(Bytes::new(second_device_id, GGA_SECOND_DEVICE), at(1));

    let effects = core
        .apply(Bytes::new(first_device_id, PFLAA_A), at(2))
        .effects;
    let delta = traffic_delta(&effects);
    let [target] = delta.upserts.as_slice() else {
        panic!("one accepted observation should produce one upsert");
    };

    assert_abs_diff_eq!(target.position.latitude_degrees, 51.009, epsilon = 1e-3);
    assert_abs_diff_eq!(target.position.longitude_degrees, 7.003, epsilon = 1e-3);
    assert_some_eq!(target.altitude_msl_meters, 350.0);
}

#[test]
fn traffic_selects_horizontal_and_vertical_references_independently() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4354)),
        ],
    });
    let first_device_id = device_id(&core, 0);
    let second_device_id = device_id(&core, 1);
    core.apply(Bytes::new(first_device_id, RMC), at(0));
    core.apply(Bytes::new(first_device_id, GGA), at(1));
    core.apply(Bytes::new(second_device_id, RMC_SECOND_DEVICE), at(1));

    let effects = core
        .apply(Bytes::new(second_device_id, PFLAA_A), at(3))
        .effects;
    let delta = traffic_delta(&effects);
    let [target] = delta.upserts.as_slice() else {
        panic!("one accepted observation should produce one upsert");
    };

    assert_abs_diff_eq!(target.position.latitude_degrees, 51.009, epsilon = 1e-3);
    assert_abs_diff_eq!(target.position.longitude_degrees, 7.003, epsilon = 1e-3);
    assert_some_eq!(target.altitude_msl_meters, 250.0);
}

#[test]
fn accepted_traffic_holds_its_absolute_position_when_ownship_moves() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));
    core.apply(Bytes::new(device_id, PFLAA_A), at(1));
    let accepted = traffic_snapshot(&core);
    let [accepted] = accepted.as_slice() else {
        panic!("the accepted target should be in the snapshot");
    };
    let accepted_position = accepted.position;

    core.apply(Bytes::new(device_id, RMC_SECOND_DEVICE), at(2));

    let held = traffic_snapshot(&core);
    let [held] = held.as_slice() else {
        panic!("the accepted target should remain in the snapshot");
    };
    assert_eq!(held.position, accepted_position);
}

#[test]
fn missing_required_traffic_fields_do_not_change_an_existing_target() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));
    core.apply(Bytes::new(device_id, PFLAA_A), at(1));
    let accepted = traffic_snapshot(&core);
    let [accepted] = accepted.as_slice() else {
        panic!("the accepted target should be in the snapshot");
    };
    let accepted = accepted.clone();

    let mut input = PFLAA_A_MISSING_EAST.to_vec();
    input.extend_from_slice(PFLAA_B);
    let effects = core.apply(Bytes::new(device_id, input), at(2)).effects;

    let delta = traffic_delta(&effects);
    let [upsert] = delta.upserts.as_slice() else {
        panic!("the usable observation after the ignored one should be published");
    };
    assert_eq!(upsert.id, "flarm:DEF456");
    let snapshot = traffic_snapshot(&core);
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0], accepted);
}

#[test]
fn batches_all_traffic_changes_from_one_bytes_input() {
    let (mut core, device_id) = core_with_external_device();
    let mut input = RMC.to_vec();
    input.extend_from_slice(PFLAA_A);
    input.extend_from_slice(PFLAA_B);

    let effects = core.apply(Bytes::new(device_id, input), at(100)).effects;

    let [
        Effect::Emit(Topic::Instruments(_)),
        Effect::Emit(Topic::Traffic(TrafficUpdate::Delta(delta))),
    ] = effects.as_slice()
    else {
        panic!("one input should emit instruments before one traffic delta");
    };
    assert_eq!(delta.upserts.len(), 2);
    assert!(delta.removed.is_empty());
}

#[test]
fn one_bytes_input_publishes_only_the_final_upsert_for_each_target() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));
    let mut input = PFLAA_A.to_vec();
    input.extend_from_slice(PFLAA_A_REPLACEMENT);

    let effects = core.apply(Bytes::new(device_id, input), at(1)).effects;
    let delta = traffic_delta(&effects);

    let [target] = delta.upserts.as_slice() else {
        panic!("the final observation should replace the earlier upsert");
    };
    assert_eq!(delta.upserts, traffic_snapshot(&core));
    assert_eq!(target.id, "icao:ABC123");
    assert!(delta.removed.is_empty());
}

#[test]
fn later_device_input_replaces_the_previous_target_with_the_same_id() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: vec![
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4353)),
            device_config(true, ConnectionSpec::tcp("127.0.0.1", 4354)),
        ],
    });
    let first_device_id = device_id(&core, 0);
    let second_device_id = device_id(&core, 1);
    core.apply(Bytes::new(first_device_id, RMC), at(0));
    core.apply(Bytes::new(second_device_id, RMC_SECOND_DEVICE), at(1));
    core.apply(Bytes::new(first_device_id, PFLAA_A), at(2));

    let effects = core
        .apply(Bytes::new(second_device_id, PFLAA_A), at(3))
        .effects;
    let delta = traffic_delta(&effects);
    let [target] = delta.upserts.as_slice() else {
        panic!("the later device input should replace the target");
    };
    assert_abs_diff_eq!(target.position.latitude_degrees, 51.009, epsilon = 1e-3);
    assert_abs_diff_eq!(target.position.longitude_degrees, 7.003, epsilon = 1e-3);
}

#[test]
fn device_disconnection_does_not_remove_traffic() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));
    core.apply(Bytes::new(device_id, PFLAA_A), at(1));

    let input = ConnectionChanged::new(device_id, ConnectionState::Disconnected);
    let effects = core.apply(input, at(2)).effects;

    assert!(effects.is_empty());
    assert_eq!(traffic_snapshot(&core).len(), 1);
}

#[test]
fn stale_tick_emits_one_complete_stale_upsert() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));
    core.apply(Bytes::new(device_id, PFLAA_A), at(100));

    let effects = core.apply(Tick, at(5_100)).effects;
    let delta = traffic_delta(&effects);

    let [target] = delta.upserts.as_slice() else {
        panic!("the stale transition should upsert the complete target");
    };
    assert!(target.stale);
    assert_eq!(target.id, "icao:ABC123");
    assert_abs_diff_eq!(target.position.latitude_degrees, 50.832, epsilon = 1e-3);
    assert!(delta.removed.is_empty());
}

#[test]
fn removal_tick_emits_one_target_id() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));
    core.apply(Bytes::new(device_id, PFLAA_A), at(100));

    let effects = core.apply(Tick, at(30_100)).effects;
    let delta = traffic_delta(&effects);

    assert!(delta.upserts.is_empty());
    assert_eq!(delta.removed.len(), 1);
    assert_eq!(delta.removed, vec!["icao:ABC123"]);
}

#[test]
fn fresh_observation_after_a_stale_tick_emits_a_fresh_upsert() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));
    core.apply(Bytes::new(device_id, PFLAA_A), at(100));
    core.apply(Tick, at(5_100));

    let effects = core
        .apply(Bytes::new(device_id, PFLAA_A), at(5_200))
        .effects;
    let delta = traffic_delta(&effects);

    let [target] = delta.upserts.as_slice() else {
        panic!("the fresh observation should replace the stale target");
    };
    assert!(!target.stale);
}

#[test]
fn input_without_a_traffic_change_emits_no_traffic_topic() {
    let (mut core, device_id) = core_with_external_device();
    let mut input = RMC.to_vec();
    input.extend_from_slice(PFLAA_A_MISSING_EAST);

    let effects = core.apply(Bytes::new(device_id, input), at(100)).effects;

    assert_matches!(effects.as_slice(), [Effect::Emit(Topic::Instruments(_))]);
}

#[test]
fn traffic_without_an_ownship_position_is_ignored() {
    let (mut core, device_id) = core_with_external_device();

    let effects = core.apply(Bytes::new(device_id, PFLAA_A), at(100)).effects;

    assert!(effects.is_empty());
    assert!(traffic_snapshot(&core).is_empty());
}

#[test]
fn new_core_exposes_an_empty_traffic_snapshot() {
    let core = Core::new(SettingsSnapshot::default());

    assert!(
        core.topics()
            .contains(&Topic::Traffic(TrafficUpdate::Snapshot(Vec::new())))
    );
}

#[test]
fn tick_emits_nothing() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(100));

    assert_eq!(core.apply(Tick, at(200)).effects, vec![]);
}

#[test]
fn flarmnet_updates_existing_targets_without_refreshing_report_age() {
    let (mut core, device_id) = core_with_external_device();
    core.apply(Bytes::new(device_id, RMC), at(0));
    core.apply(Bytes::new(device_id, PFLAA_A), at(0));
    core.apply(Tick, at(5_000));
    let before = traffic_snapshot(&core)[0].clone();
    assert!(before.stale);

    let json = br#"[{"flarm_id":"ABC123","call_sign":"EL"}]"#;
    let database = Arc::new(assert_ok!(FlarmnetDatabase::from_json(json)));
    let update = core.apply(ReplaceFlarmnetDatabase(database.clone()), at(6_000));
    let [Effect::Emit(Topic::Traffic(TrafficUpdate::Snapshot(targets)))] =
        update.effects.as_slice()
    else {
        panic!("Database replacement should refresh existing traffic");
    };
    let mut target = targets[0].clone();
    assert_eq!(assert_some!(target.flarmnet.take()).call_sign, "EL");
    assert_eq!(target, before);
    assert_eq!(&traffic_snapshot(&core), targets);
    let unchanged = core.apply(ReplaceFlarmnetDatabase(database), at(7_000));
    assert!(unchanged.effects.is_empty());

    let empty = Arc::new(FlarmnetDatabase::default());
    core.apply(ReplaceFlarmnetDatabase(empty), at(8_000));
    assert_none!(&traffic_snapshot(&core)[0].flarmnet);
    core.apply(Tick, at(30_000));
    assert!(traffic_snapshot(&core).is_empty());
}

#[test]
fn flarmnet_enriches_report_and_expiry_deltas() {
    let (mut core, device_id) = core_with_external_device();
    let json = br#"[{"flarm_id":"ABC123","registration":"D-TEST"}]"#;
    let database = Arc::new(assert_ok!(FlarmnetDatabase::from_json(json)));
    core.apply(ReplaceFlarmnetDatabase(database), at(0));
    core.apply(Bytes::new(device_id, RMC), at(0));
    let report = core.apply(Bytes::new(device_id, PFLAA_A), at(0));
    let reported = traffic_delta(&report.effects).upserts[0].clone();
    assert_eq!(assert_some!(&reported.flarmnet).registration, "D-TEST");
    let expiry = core.apply(Tick, at(5_000));
    let stale = traffic_delta(&expiry.effects).upserts[0].clone();
    assert_eq!(stale.flarmnet, reported.flarmnet);
    assert!(stale.stale);
}

#[test]
fn climb_uses_altitude_changes_and_rejects_stale_ownship_altitude() {
    let (mut core, device) = core_with_external_device();
    core.apply(Bytes::new(device, GGA), at(0));
    core.apply(Bytes::new(device, PFLAA_A), at(0));
    assert_none!(traffic_snapshot(&core)[0].climb);
    core.apply(Bytes::new(device, PFLAA_A_REPLACEMENT), at(1_000));
    let climb = assert_some!(traffic_snapshot(&core)[0].climb);
    assert_eq!(climb.average_20s, Speed::from_meters_per_second(50.0));
    assert_eq!(climb.average_30s, Speed::from_meters_per_second(50.0));
    assert_eq!(climb.normalized_ema, Speed::from_meters_per_second(50.0));
    insta::assert_json_snapshot!(climb);
    core.apply(Bytes::new(device, PFLAA_A), at(3_000));
    assert_none!(traffic_snapshot(&core)[0].climb);
    core.apply(Bytes::new(device, GGA), at(5_000));
    core.apply(Bytes::new(device, PFLAA_A), at(5_000));
    assert_eq!(
        assert_some!(traffic_snapshot(&core)[0].climb).average_20s,
        Speed::ZERO
    );
}

#[test]
fn climb_history_survives_target_removal_and_expires_after_sixty_seconds() {
    let (mut core, device) = core_with_external_device();
    core.apply(Bytes::new(device, GGA), at(0));
    core.apply(Bytes::new(device, PFLAA_A), at(0));
    core.apply(Tick, at(30_000));
    assert!(traffic_snapshot(&core).is_empty());
    core.apply(Tick, at(60_000));
    core.apply(Bytes::new(device, GGA), at(60_000));
    core.apply(Bytes::new(device, PFLAA_A_REPLACEMENT), at(60_000));
    let climb = assert_some!(traffic_snapshot(&core)[0].climb);
    assert_abs_diff_eq!(
        climb.average_20s,
        Speed::from_meters_per_second(50.0) / 60.0,
        epsilon = 1e-12
    );
    core.apply(Tick, at(120_001));
    core.apply(Bytes::new(device, GGA), at(120_001));
    core.apply(Bytes::new(device, PFLAA_A), at(120_001));
    assert_none!(traffic_snapshot(&core)[0].climb);
}

#[test]
fn climb_resets_when_reporting_device_or_fallback_altitude_source_changes() {
    let mut core = Core::new(SettingsSnapshot {
        settings: Settings::default(),
        external_devices: (4353..4356)
            .map(|port| device_config(true, ConnectionSpec::tcp("127.0.0.1", port)))
            .collect(),
    });
    let first = device_id(&core, 0);
    let second = device_id(&core, 1);
    let reporter = device_id(&core, 2);
    core.apply(Bytes::new(first, GGA), at(0));
    core.apply(Bytes::new(reporter, PFLAA_A), at(0));
    core.apply(Bytes::new(reporter, PFLAA_A_REPLACEMENT), at(1_000));
    assert_some!(traffic_snapshot(&core)[0].climb);
    core.apply(Bytes::new(reporter, PFLAA_A), at(3_000));
    assert_none!(traffic_snapshot(&core)[0].climb);
    core.apply(Bytes::new(second, GGA_SECOND_DEVICE), at(4_000));
    core.apply(Bytes::new(reporter, PFLAA_A), at(4_000));
    assert_none!(traffic_snapshot(&core)[0].climb);
    core.apply(Bytes::new(reporter, PFLAA_A_REPLACEMENT), at(5_000));
    assert_some!(traffic_snapshot(&core)[0].climb);
    core.apply(Bytes::new(second, PFLAA_A), at(6_000));
    assert_none!(traffic_snapshot(&core)[0].climb);
}
