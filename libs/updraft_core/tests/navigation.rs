use claims::{assert_none, assert_ok, assert_some};
use updraft_core::{
    Core, Fix, InternalGps, NavigationTarget, SetNavigationTarget, SettingsSnapshot, Tick,
    Timestamp, Topic,
};
use updraft_geo::LatLon;
use updraft_units::{Angle, EllipsoidAltitude, Length};

fn navigation(core: &Core) -> Option<updraft_core::Navigation> {
    core.topics()
        .into_iter()
        .find_map(|topic| match topic {
            Topic::Navigation(value) => Some(value),
            _ => None,
        })
        .flatten()
}

#[test]
fn goto_retains_target_at_arrival_and_marks_stale_ownship() {
    let mut core = Core::new(SettingsSnapshot::default());
    let target = NavigationTarget::Waypoint {
        name: "Home".into(),
        latitude_degrees: 0.,
        longitude_degrees: 0.,
        elevation_meters: 100.,
    };
    assert_ok!(
        core.apply(SetNavigationTarget(Some(target.clone())), at(0))
            .response
    );
    assert_none!(assert_some!(navigation(&core)).guidance);
    core.apply(
        InternalGps::new(fix(Some(Angle::from_degrees(90.)), None)),
        at(0),
    );
    let value = assert_some!(navigation(&core));
    assert_eq!(value.target, target);
    assert_eq!(assert_some!(value.guidance).distance_meters, 0.);
    core.apply(Tick, at(4000));
    assert!(assert_some!(assert_some!(navigation(&core)).guidance).stale);
    assert_ok!(core.apply(SetNavigationTarget(None), at(4000)).response);
    assert_none!(navigation(&core));
}

#[test]
fn invalid_target_does_not_replace_navigation() {
    let mut core = Core::new(SettingsSnapshot::default());
    let target = NavigationTarget::Waypoint {
        name: "Home".into(),
        latitude_degrees: 91.,
        longitude_degrees: 0.,
        elevation_meters: 100.,
    };
    claims::assert_err!(
        core.apply(SetNavigationTarget(Some(target)), at(0))
            .response
    );
    assert_none!(navigation(&core));
}

#[test]
fn goto_publishes_guidance_updates_and_keeps_snapshot_after_catalog_change() {
    let mut core = Core::new(SettingsSnapshot::default());
    core.apply(
        InternalGps::new(fix(Some(Angle::from_degrees(350.)), None)),
        at(0),
    );
    let update = core.apply(
        SetNavigationTarget(Some(NavigationTarget::Waypoint {
            name: "North".into(),
            latitude_degrees: 1.,
            longitude_degrees: 0.,
            elevation_meters: 0.,
        })),
        at(0),
    );
    assert_ok!(update.response);
    assert_eq!(update.effects.len(), 2);
    let expected = assert_some!(navigation(&core));
    let guidance = assert_some!(expected.guidance);
    assert_eq!(guidance.bearing_degrees, 0.);
    claims::assert_some_eq!(guidance.relative_bearing_degrees, 10.);
    core.apply(
        updraft_core::ReplaceWaypointCatalog(Default::default()),
        at(0),
    );
    assert_eq!(assert_some!(navigation(&core)), expected);
}

#[test]
fn waypoint_arrival_uses_fused_altitude_and_current_reserve() {
    let mut core = Core::new(SettingsSnapshot::default());
    assert_ok!(
        core.apply(
            SetNavigationTarget(Some(NavigationTarget::Waypoint {
                name: "Home".into(),
                latitude_degrees: 0.,
                longitude_degrees: 0.,
                elevation_meters: 100.,
            })),
            at(0)
        )
        .response
    );
    let altitude = EllipsoidAltitude::new(Length::from_meters(1000.));
    core.apply(InternalGps::new(fix(None, Some(altitude))), at(0));
    let first = assert_some!(assert_some!(navigation(&core)).arrival);
    core.apply(
        updraft_core::SetArrivalReserve {
            reserve: assert_ok!(updraft_core::ArrivalReserve::try_from(500.)),
        },
        at(0),
    );
    let second = assert_some!(assert_some!(navigation(&core)).arrival);
    assert_eq!(first.margin_meters - second.margin_meters, 300.);
    assert!(!second.stale);
    core.apply(Tick, at(4000));
    assert!(assert_some!(assert_some!(navigation(&core)).arrival).stale);
}

#[test]
fn map_target_accepts_only_its_terrain_result_and_resets_elevation_on_replacement() {
    let mut core = Core::new(SettingsSnapshot::default());
    let altitude = EllipsoidAltitude::new(Length::from_meters(1000.));
    core.apply(InternalGps::new(fix(None, Some(altitude))), at(0));
    let target = NavigationTarget::MapPosition {
        latitude_degrees: 0.,
        longitude_degrees: 0.,
    };
    assert_ok!(
        core.apply(SetNavigationTarget(Some(target)), at(0))
            .response
    );
    assert_some!(assert_some!(navigation(&core)).guidance);
    assert_none!(assert_some!(navigation(&core)).arrival);
    core.apply(
        updraft_core::NavigationElevation {
            position: updraft_core::LatLon {
                latitude_degrees: 1.,
                longitude_degrees: 0.,
            },
            meters: Some(100.),
        },
        at(0),
    );
    assert_none!(assert_some!(navigation(&core)).arrival);
    let position = updraft_core::LatLon {
        latitude_degrees: 0.,
        longitude_degrees: 0.,
    };
    core.apply(
        updraft_core::NavigationElevation {
            position,
            meters: Some(100.),
        },
        at(0),
    );
    assert_some!(assert_some!(navigation(&core)).arrival);
    assert_ok!(
        core.apply(
            SetNavigationTarget(Some(NavigationTarget::MapPosition {
                latitude_degrees: 0.,
                longitude_degrees: 0.
            })),
            at(0)
        )
        .response
    );
    assert_some!(assert_some!(navigation(&core)).arrival);
    assert_ok!(
        core.apply(
            SetNavigationTarget(Some(NavigationTarget::MapPosition {
                latitude_degrees: 1.,
                longitude_degrees: 0.
            })),
            at(0)
        )
        .response
    );
    core.apply(
        updraft_core::NavigationElevation {
            position,
            meters: Some(100.),
        },
        at(0),
    );
    assert_none!(assert_some!(navigation(&core)).arrival);
}

fn at(millis: u64) -> Timestamp {
    Timestamp::from_millis(millis)
}

fn fix(track: Option<Angle>, altitude_ellipsoid: Option<EllipsoidAltitude>) -> Fix {
    Fix {
        position: LatLon::from_degrees(0., 0.),
        altitude_ellipsoid,
        track,
        ground_speed: None,
        fix_time: None,
    }
}
