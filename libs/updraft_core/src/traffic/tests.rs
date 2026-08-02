use super::*;
use crate::time::Timestamp;
use approx::assert_abs_diff_eq;
use claims::{assert_none, assert_some, assert_some_eq};
use updraft_geo::LatLon as GeoLatLon;
use updraft_nmea::{FlarmAircraftType, FlarmAlarmLevel, FlarmId, FlarmIdType, FlarmSource, Pflaa};
use updraft_units::{Angle, Length, MslAltitude, Speed};

fn pflaa() -> Pflaa {
    Pflaa {
        alarm_level: FlarmAlarmLevel::None,
        relative_north: Some(Length::from_meters(1_000.0)),
        relative_east: Some(Length::from_meters(1_000.0)),
        relative_vertical: Some(Length::from_meters(150.0)),
        id_type: Some(FlarmIdType::Flarm),
        id: Some(FlarmId {
            address: 0xDD8F12,
            callsign: Some("IGNORED".into()),
        }),
        track: Some(Angle::from_degrees(180.0)),
        turn_rate: Some(2.0),
        ground_speed: Some(Speed::from_meters_per_second(30.0)),
        climb_rate: Some(Speed::from_meters_per_second(-1.4)),
        aircraft_type: FlarmAircraftType::Glider,
        no_track: Some(true),
        source: Some(FlarmSource::Flarm),
        rssi: Some(-58.5),
    }
}

fn target(value: u32) -> TrafficTarget {
    TrafficTarget {
        id: TrafficTargetId::new(TrafficTargetIdType::Flarm, value),
        position: GeoLatLon::from_degrees(50.823, 6.186),
        altitude_msl: Some(MslAltitude::new(Length::from_meters(200.0))),
        traffic_type: TrafficType::Glider,
        track: Some(Angle::from_degrees(180.0)),
        alarm_level: TrafficAlarmLevel::None,
        stale: false,
    }
}

#[test]
fn projects_pflaa_with_typed_reference_values() {
    let target_pflaa = pflaa();
    let ownship_position = GeoLatLon::from_degrees(50.0, 6.0);
    let ownship_altitude = Some(MslAltitude::new(Length::from_meters(200.0)));

    let target = assert_some!(target_from_pflaa(
        &target_pflaa,
        ownship_position,
        ownship_altitude
    ));
    let distance = ownship_position.distance(target.position);
    let bearing = ownship_position.bearing(target.position);

    assert_eq!(
        target.id,
        TrafficTargetId::new(TrafficTargetIdType::Flarm, 0xDD8F12)
    );
    assert_abs_diff_eq!(distance.as_meters(), 1_414.214, epsilon = 0.001);
    assert_abs_diff_eq!(bearing.as_degrees(), 45.0, epsilon = 0.001);
    assert_some_eq!(
        target.altitude_msl,
        MslAltitude::new(Length::from_meters(350.0))
    );
    assert_eq!(target.traffic_type, TrafficType::Glider);
    assert_some_eq!(target.track, Angle::from_degrees(180.0));
    assert_eq!(target.alarm_level, TrafficAlarmLevel::None);
    assert!(!target.stale);
}

#[test]
fn normalizes_unsupported_pflaa_values() {
    let mut target_pflaa = pflaa();
    target_pflaa.id_type = Some(FlarmIdType::Other(7));
    target_pflaa.aircraft_type = FlarmAircraftType::Other(14);
    target_pflaa.alarm_level = FlarmAlarmLevel::Other(4);

    let target = assert_some!(target_from_pflaa(
        &target_pflaa,
        GeoLatLon::from_degrees(51.0, 7.0),
        None
    ));

    assert_eq!(
        target.id,
        TrafficTargetId::new(TrafficTargetIdType::Other(7), 0xDD8F12)
    );
    assert_eq!(target.traffic_type, TrafficType::Unknown);
    assert_eq!(target.alarm_level, TrafficAlarmLevel::Unknown);
}

#[test]
fn ignores_pflaa_without_required_map_fields() {
    let ownship_position = GeoLatLon::from_degrees(51.0, 7.0);

    let mut missing_id_type = pflaa();
    missing_id_type.id_type = None;
    assert_none!(target_from_pflaa(&missing_id_type, ownship_position, None));

    let mut missing_id = pflaa();
    missing_id.id = None;
    assert_none!(target_from_pflaa(&missing_id, ownship_position, None));

    let mut missing_north = pflaa();
    missing_north.relative_north = None;
    assert_none!(target_from_pflaa(&missing_north, ownship_position, None));

    let mut missing_east = pflaa();
    missing_east.relative_east = None;
    assert_none!(target_from_pflaa(&missing_east, ownship_position, None));
}

#[test]
fn projects_pflaa_without_optional_track_or_altitude() {
    let ownship_position = GeoLatLon::from_degrees(50.0, 6.0);
    let ownship_altitude = Some(MslAltitude::new(Length::from_meters(200.0)));
    let mut missing_track = pflaa();
    missing_track.track = None;
    let target = assert_some!(target_from_pflaa(
        &missing_track,
        ownship_position,
        ownship_altitude
    ));
    assert_none!(target.track);

    let mut missing_vertical = pflaa();
    missing_vertical.relative_vertical = None;
    let target = assert_some!(target_from_pflaa(
        &missing_vertical,
        ownship_position,
        ownship_altitude
    ));
    assert_none!(target.altitude_msl);

    let target = assert_some!(target_from_pflaa(&pflaa(), ownship_position, None));
    assert_none!(target.altitude_msl);
}

#[test]
fn replaces_a_target_and_resets_its_stale_state() {
    let mut state = TrafficState::default();
    let mut changes = TrafficChanges::default();
    let original = target(1);
    let id = original.id;
    state.observe(original, Timestamp::from_millis(0), &mut changes);
    let stale = state.expire(Timestamp::from_millis(5_000));
    assert!(stale.upserts[&id].stale);

    let mut replacement = target(1);
    replacement.position = GeoLatLon::from_degrees(51.0, 7.0);
    replacement.altitude_msl = None;
    replacement.traffic_type = TrafficType::TowPlane;
    replacement.track = None;
    replacement.alarm_level = TrafficAlarmLevel::Urgent;
    state.observe(replacement, Timestamp::from_millis(6_000), &mut changes);

    assert_eq!(changes.upserts[&id], replacement);
    assert!(!changes.upserts[&id].stale);
}

#[test]
fn keeps_a_target_fresh_before_the_stale_boundary() {
    let mut state = TrafficState::default();
    let mut changes = TrafficChanges::default();
    let target = target(1);
    state.observe(target, Timestamp::from_millis(0), &mut changes);

    let expired = state.expire(Timestamp::from_millis(4_999));

    assert!(expired.upserts.is_empty());
    assert!(expired.removed.is_empty());
    assert_eq!(state.snapshot(), vec![target]);
}

#[test]
fn identical_observation_refreshes_the_stale_deadline_without_an_upsert() {
    let mut state = TrafficState::default();
    let target = target(1);
    state.observe(
        target,
        Timestamp::from_millis(0),
        &mut TrafficChanges::default(),
    );

    let mut changes = TrafficChanges::default();
    state.observe(target, Timestamp::from_millis(1_000), &mut changes);

    assert!(changes.upserts.is_empty());
    assert!(changes.removed.is_empty());
    assert!(
        state
            .expire(Timestamp::from_millis(5_999))
            .upserts
            .is_empty()
    );
    assert!(state.expire(Timestamp::from_millis(6_000)).upserts[&target.id].stale);
}

#[test]
fn marks_and_removes_targets_at_exact_boundaries() {
    let mut state = TrafficState::default();
    let mut changes = TrafficChanges::default();
    let target = target(1);
    state.observe(target, Timestamp::from_millis(0), &mut changes);
    assert!(!changes.upserts[&target.id].stale);

    let stale = state.expire(Timestamp::from_millis(5_000));
    assert!(stale.upserts[&target.id].stale);
    assert!(stale.removed.is_empty());

    let repeated_stale = state.expire(Timestamp::from_millis(6_000));
    assert!(repeated_stale.upserts.is_empty());
    assert!(repeated_stale.removed.is_empty());

    let retained = state.expire(Timestamp::from_millis(29_999));
    assert!(retained.upserts.is_empty());
    assert!(retained.removed.is_empty());
    assert_eq!(state.snapshot().len(), 1);

    let removed = state.expire(Timestamp::from_millis(30_000));
    assert!(removed.upserts.is_empty());
    assert_eq!(
        removed.removed.into_iter().collect::<Vec<_>>(),
        vec![target.id]
    );
    assert!(state.snapshot().is_empty());
}

#[test]
fn removal_wins_when_the_first_tick_crosses_both_boundaries() {
    let mut state = TrafficState::default();
    let mut changes = TrafficChanges::default();
    let target = target(1);
    state.observe(target, Timestamp::from_millis(0), &mut changes);

    let expired = state.expire(Timestamp::from_millis(30_000));

    assert!(expired.upserts.is_empty());
    assert_eq!(
        expired.removed.into_iter().collect::<Vec<_>>(),
        vec![target.id]
    );
}

#[test]
fn orders_snapshots_and_deltas_by_target_id() {
    let mut state = TrafficState::default();
    let mut changes = TrafficChanges::default();
    for value in [3, 1, 2] {
        state.observe(target(value), Timestamp::from_millis(0), &mut changes);
    }

    let snapshot_ids = state
        .snapshot()
        .into_iter()
        .map(|target| target.id.value)
        .collect::<Vec<_>>();
    let upsert_ids = changes
        .upserts
        .keys()
        .map(|id| id.value)
        .collect::<Vec<_>>();
    assert_eq!(snapshot_ids, vec![1, 2, 3]);
    assert_eq!(upsert_ids, vec![1, 2, 3]);

    let expired = state.expire(Timestamp::from_millis(30_000));
    let removed_ids = expired
        .removed
        .into_iter()
        .map(|id| id.value)
        .collect::<Vec<_>>();
    assert_eq!(removed_ids, vec![1, 2, 3]);
}

#[test]
fn projects_ordered_and_mutually_exclusive_traffic_deltas() {
    let mut changes = TrafficChanges::default();
    changes.upsert(target(3));
    changes.upsert(target(1));
    changes.upsert(target(2));
    changes.remove(target(2).id);
    changes.upsert(target(2));
    changes.remove(target(4).id);

    let delta: TrafficDelta = changes.into();
    let upsert_ids = delta
        .upserts
        .into_iter()
        .map(|target| target.id)
        .collect::<Vec<_>>();

    assert_eq!(
        upsert_ids,
        vec!["flarm:000001", "flarm:000002", "flarm:000003"]
    );
    assert_eq!(delta.removed, vec!["flarm:000004"]);
}

#[test]
fn formats_canonical_wire_target_ids() {
    for (id, expected) in [
        (
            TrafficTargetId::new(TrafficTargetIdType::Random, 0x000001),
            "random:000001",
        ),
        (
            TrafficTargetId::new(TrafficTargetIdType::Icao, 0xABCDEF),
            "icao:ABCDEF",
        ),
        (
            TrafficTargetId::new(TrafficTargetIdType::Flarm, 0x000123),
            "flarm:000123",
        ),
        (
            TrafficTargetId::new(TrafficTargetIdType::Other(7), 0x000123),
            "other:7:000123",
        ),
    ] {
        assert_eq!(id.to_string(), expected);
    }
}
