use claims::{assert_err, assert_ok};
use updraft_core::*;

fn waypoint(name: &str, latitude_degrees: f64) -> NavigationTarget {
    NavigationTarget::Waypoint {
        name: name.to_owned(),
        latitude_degrees,
        longitude_degrees: 6.,
        elevation_meters: 100.,
    }
}

fn pins(core: &Core) -> Vec<PinnedTarget> {
    core.topics()
        .into_iter()
        .find_map(|topic| match topic {
            Topic::PinnedTargets(pins) => Some(pins),
            _ => None,
        })
        .unwrap()
}

#[test]
fn pins_keep_order_snapshots_and_identity_independently_of_navigation() {
    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::default();
    let first = waypoint("Home", 50.);
    let saved = assert_ok!(core.apply(PinTarget(first.clone()), at).response);
    let id = saved[0].id;
    let mut duplicate = waypoint("Home", 50.00005);
    if let NavigationTarget::Waypoint {
        elevation_meters, ..
    } = &mut duplicate
    {
        *elevation_meters = 200.;
    }
    assert_eq!(
        assert_ok!(core.apply(PinTarget(duplicate.clone()), at).response),
        saved
    );
    assert_ok!(
        core.apply(PinTarget(waypoint("Alternate", 50.)), at)
            .response
    );
    assert_ok!(
        core.apply(SetNavigationTarget(Some(duplicate)), at)
            .response
    );
    assert!(pins(&core)[0].primary);
    assert!(!pins(&core)[1].primary);
    assert_ok!(core.apply(SetNavigationTarget(None), at).response);
    assert!(!pins(&core)[0].primary);
    assert_eq!(pins(&core)[0].navigation.target, first);
    assert_ok!(core.apply(UnpinTarget(id), at).response);
    assert_ok!(core.apply(PinTarget(first), at).response);
    assert_ne!(pins(&core)[1].id, id);
    let previous = pins(&core);
    assert_err!(core.apply(PinTarget(waypoint("Invalid", 91.)), at).response);
    assert_eq!(pins(&core), previous);
}

#[test]
fn restore_validates_the_whole_list_and_preserves_ids() {
    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::default();
    let saved = vec![SavedPinnedTarget {
        id: 12,
        target: waypoint("Home", 50.),
    }];
    assert_ok!(core.apply(RestorePinnedTargets(saved.clone()), at).response);
    assert_eq!(pins(&core)[0].id, 12);
    assert_err!(
        core.apply(
            RestorePinnedTargets(vec![saved[0].clone(), saved[0].clone()]),
            at
        )
        .response
    );
    assert_eq!(pins(&core).len(), 1);
    assert_ok!(core.apply(PinTarget(waypoint("Next", 51.)), at).response);
    assert_eq!(pins(&core)[1].id, 13);
}

#[test]
fn matching_uses_names_types_and_coordinate_tolerance() {
    let target = waypoint("Home", 50.);
    assert!(target.matches(&waypoint("Home", 50.00009)));
    assert!(!target.matches(&waypoint("Home", 50.00011)));
    assert!(!target.matches(&waypoint("Other", 50.)));
    assert!(!target.matches(&NavigationTarget::MapPosition {
        latitude_degrees: 50.,
        longitude_degrees: 6.
    }));
    let west = NavigationTarget::MapPosition {
        latitude_degrees: 0.,
        longitude_degrees: -180.,
    };
    let east = NavigationTarget::MapPosition {
        latitude_degrees: 0.,
        longitude_degrees: 180.,
    };
    assert!(west.matches(&east));
}

#[test]
fn pinned_map_arrival_uses_only_matching_terrain_results() {
    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::default();
    core.apply(
        InternalGps::new(Fix {
            position: updraft_geo::LatLon::from_degrees(0., 0.),
            altitude_ellipsoid: Some(updraft_units::EllipsoidAltitude::new(
                updraft_units::Length::from_meters(1000.),
            )),
            track: None,
            ground_speed: None,
            fix_time: None,
        }),
        at,
    );
    let target = NavigationTarget::MapPosition {
        latitude_degrees: 0.,
        longitude_degrees: 0.,
    };
    let saved = assert_ok!(core.apply(PinTarget(target.clone()), at).response);
    let id = saved[0].id;
    let position = LatLon {
        latitude_degrees: 0.,
        longitude_degrees: 0.,
    };
    claims::assert_some!(pins(&core)[0].navigation.guidance);
    claims::assert_none!(pins(&core)[0].navigation.arrival);
    core.apply(
        PinnedTargetElevation {
            id,
            position,
            meters: Some(100.),
        },
        at,
    );
    let arrival = claims::assert_some!(pins(&core)[0].navigation.arrival);
    core.apply(
        SetArrivalReserve {
            reserve: assert_ok!(ArrivalReserve::try_from(500.)),
        },
        at,
    );
    assert_eq!(
        arrival.margin_meters
            - claims::assert_some!(pins(&core)[0].navigation.arrival).margin_meters,
        300.
    );
    assert_ok!(core.apply(UnpinTarget(id), at).response);
    assert_ok!(core.apply(PinTarget(target), at).response);
    core.apply(
        PinnedTargetElevation {
            id,
            position,
            meters: Some(100.),
        },
        at,
    );
    claims::assert_none!(pins(&core)[0].navigation.arrival);
    let id = pins(&core)[0].id;
    core.apply(
        PinnedTargetElevation {
            id,
            position: LatLon {
                latitude_degrees: 1.,
                ..position
            },
            meters: Some(100.),
        },
        at,
    );
    claims::assert_none!(pins(&core)[0].navigation.arrival);
    core.apply(
        PinnedTargetElevation {
            id,
            position,
            meters: Some(100.),
        },
        at,
    );
    core.apply(
        PinnedTargetElevation {
            id,
            position,
            meters: None,
        },
        at,
    );
    claims::assert_none!(pins(&core)[0].navigation.arrival);
}
