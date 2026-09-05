use claims::assert_ok;
use std::{collections::BTreeMap, sync::Arc};
use updraft_core::{
    Core, GetWaypointCatalog, ReplaceWaypointCatalog, SettingsSnapshot, Timestamp, Topic,
    WaypointCatalog,
};
use updraft_waypoint::WaypointDataset;

#[test]
fn glide_snapshot_keeps_the_catalog_flight_state_and_performance_from_one_query() {
    use claims::{assert_none, assert_some};
    use updraft_core::{
        ArrivalReserve, Ballast, Bugs, Fix, GetGlideSnapshot, InternalGps, MacCready, PolarId,
        SetArrivalReserve, SetBallast, SetBugs, SetMacCready, SetPolar,
    };
    use updraft_geo::LatLon;

    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::from_millis(0);
    let before = core.apply(GetGlideSnapshot, at);
    assert_eq!(before.effects, vec![]);
    let before = before.response;
    assert_eq!(before.mac_cready, MacCready::default());
    assert_eq!(before.arrival_reserve.meters(), 200.);
    assert_eq!(before.waypoints.generation, 0);
    assert_eq!(before.polar.total_mass().as_kilograms(), 336.);
    assert_none!(before.instruments.gps);

    let bytes = b"name,code,country,lat,lon,elev,style\nField,,,5000.000N,00600.000E,100m,2\n";
    let dataset = Arc::new(assert_ok!(WaypointDataset::from_cup(bytes)));
    let catalog = Arc::new(WaypointCatalog {
        sources: BTreeMap::from([("field.cup".into(), Ok(dataset))]),
    });
    core.apply(ReplaceWaypointCatalog(catalog.clone()), at);
    let position = LatLon::from_degrees(50.1, 6.1);
    let fix = Fix {
        position,
        altitude_ellipsoid: None,
        track: None,
        ground_speed: None,
        fix_time: None,
    };
    core.apply(InternalGps::new(fix), at);
    let polar = assert_ok!(PolarId::try_from("ASK 21".to_owned()));
    core.apply(SetPolar { polar }, at);
    let reserve = assert_ok!(ArrivalReserve::try_from(300.));
    core.apply(SetArrivalReserve { reserve }, at);
    let mac_cready = assert_ok!(MacCready::try_from(1.5));
    core.apply(SetMacCready { mac_cready }, at);
    let bugs = assert_ok!(Bugs::try_from(10.));
    core.apply(SetBugs { bugs }, at);
    let ballast = assert_ok!(Ballast::try_from(100.));
    core.apply(SetBallast { ballast }, at);

    let after = core.apply(GetGlideSnapshot, at);
    assert_eq!(after.effects, vec![]);
    let after = after.response;
    assert_eq!(after.mac_cready, mac_cready);
    assert_eq!(after.waypoints.generation, 1);
    assert!(Arc::ptr_eq(&after.waypoints.catalog, &catalog));
    let expected_position = updraft_core::LatLon {
        latitude_degrees: position.latitude().as_degrees(),
        longitude_degrees: position.longitude().as_degrees(),
    };
    let gps = assert_some!(after.instruments.gps);
    assert_eq!(gps.position, expected_position);
    assert_eq!(after.arrival_reserve, reserve);
    assert_eq!(after.polar.total_mass().as_kilograms(), 638.);
    assert_eq!(after.polar.bugs(), 0.1);
    assert_eq!(before.waypoints.catalog.sources.len(), 0);
    assert_eq!(before.polar.total_mass().as_kilograms(), 336.);
    assert_none!(before.instruments.gps);
    assert_eq!(before.arrival_reserve.meters(), 200.);
}

#[test]
fn replaces_catalog_and_publishes_a_generation_without_merging_sources() {
    let bytes = b"name,code,country,lat,lon,elev,style\nField,,,5000.000N,00600.000E,100m,2\n";
    let dataset = Arc::new(assert_ok!(WaypointDataset::from_cup(bytes)));
    let catalog = Arc::new(WaypointCatalog {
        sources: BTreeMap::from([
            ("a.cup".into(), Ok(dataset.clone())),
            ("b.cup".into(), Ok(dataset)),
        ]),
    });
    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::from_millis(0);
    core.apply(ReplaceWaypointCatalog(catalog.clone()), at);
    let snapshot = core.apply(GetWaypointCatalog, at).response;
    assert!(Arc::ptr_eq(&snapshot, &catalog));
    let status = core
        .topics()
        .into_iter()
        .find_map(|topic| match topic {
            Topic::Waypoints(status) => Some(status),
            _ => None,
        })
        .unwrap();
    assert_eq!(status.generation, 1);
    assert_eq!(status.sources.len(), 2);
    core.apply(
        ReplaceWaypointCatalog(Arc::new(WaypointCatalog::default())),
        at,
    );
    assert!(
        core.apply(GetWaypointCatalog, at)
            .response
            .sources
            .is_empty()
    );
    assert_eq!(catalog.sources.len(), 2);
}
