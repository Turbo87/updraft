use claims::{assert_none, assert_ok, assert_some};
use updraft_core::{
    Core, Fix, InternalGps, NavigationTarget, SetNavigationTarget, SettingsSnapshot, Tick,
    Timestamp, Topic,
};
use updraft_geo::LatLon;
use updraft_units::Angle;

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
        core.apply(
            SetNavigationTarget(Some(target.clone())),
            Timestamp::from_millis(0)
        )
        .response
    );
    assert_none!(assert_some!(navigation(&core)).guidance);
    core.apply(
        InternalGps::new(Fix {
            position: LatLon::from_degrees(0., 0.),
            altitude_ellipsoid: None,
            track: Some(Angle::from_degrees(90.)),
            ground_speed: None,
            fix_time: None,
        }),
        Timestamp::from_millis(0),
    );
    let value = assert_some!(navigation(&core));
    assert_eq!(value.target, target);
    assert_eq!(assert_some!(value.guidance).distance_meters, 0.);
    core.apply(Tick, Timestamp::from_millis(4000));
    assert!(assert_some!(assert_some!(navigation(&core)).guidance).stale);
    assert_ok!(
        core.apply(SetNavigationTarget(None), Timestamp::from_millis(4000))
            .response
    );
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
        core.apply(SetNavigationTarget(Some(target)), Timestamp::from_millis(0))
            .response
    );
    assert_none!(navigation(&core));
}

#[test]
fn goto_publishes_guidance_updates_and_keeps_snapshot_after_catalog_change() {
    let mut core = Core::new(SettingsSnapshot::default());
    core.apply(
        InternalGps::new(Fix {
            position: LatLon::from_degrees(0., 0.),
            altitude_ellipsoid: None,
            track: Some(Angle::from_degrees(350.)),
            ground_speed: None,
            fix_time: None,
        }),
        Timestamp::from_millis(0),
    );
    let update = core.apply(
        SetNavigationTarget(Some(NavigationTarget::Waypoint {
            name: "North".into(),
            latitude_degrees: 1.,
            longitude_degrees: 0.,
            elevation_meters: 0.,
        })),
        Timestamp::from_millis(0),
    );
    assert_ok!(update.response);
    assert_eq!(update.effects.len(), 1);
    let expected = assert_some!(navigation(&core));
    let guidance = assert_some!(expected.guidance);
    assert_eq!(guidance.bearing_degrees, 0.);
    claims::assert_some_eq!(guidance.relative_bearing_degrees, 10.);
    core.apply(
        updraft_core::ReplaceWaypointCatalog(Default::default()),
        Timestamp::from_millis(0),
    );
    assert_eq!(assert_some!(navigation(&core)), expected);
}

#[test]
fn waypoint_arrival_uses_fused_altitude_and_current_reserve() {
    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::from_millis(0);
    assert_ok!(
        core.apply(
            SetNavigationTarget(Some(NavigationTarget::Waypoint {
                name: "Home".into(),
                latitude_degrees: 0.,
                longitude_degrees: 0.,
                elevation_meters: 100.,
            })),
            at
        )
        .response
    );
    core.apply(
        InternalGps::new(Fix {
            position: LatLon::from_degrees(0., 0.),
            altitude_ellipsoid: Some(updraft_units::EllipsoidAltitude::new(
                updraft_units::Length::from_meters(1000.),
            )),
            track: None,
            ground_speed: None,
            fix_time: None,
        }),
        at,
    );
    let first = assert_some!(assert_some!(navigation(&core)).arrival);
    core.apply(
        updraft_core::SetArrivalReserve {
            reserve: assert_ok!(updraft_core::ArrivalReserve::try_from(500.)),
        },
        at,
    );
    let second = assert_some!(assert_some!(navigation(&core)).arrival);
    assert_eq!(first.margin_meters - second.margin_meters, 300.);
    assert!(!second.stale);
    core.apply(Tick, Timestamp::from_millis(4000));
    assert!(assert_some!(assert_some!(navigation(&core)).arrival).stale);
}
