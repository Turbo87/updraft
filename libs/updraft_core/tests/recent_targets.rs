use claims::assert_ok;
use updraft_core::{
    Core, GetRecentTargets, NavigationTarget, PinTarget, SetNavigationTarget, SettingsSnapshot,
    Timestamp, UnpinTarget,
};

#[test]
fn recent_targets_are_bounded_deduplicated_and_include_unpinned_targets() {
    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::default();
    let target = |n| NavigationTarget::MapPosition {
        latitude_degrees: 50.,
        longitude_degrees: n as f64,
    };
    for n in 0..31 {
        assert_ok!(
            core.apply(SetNavigationTarget(Some(target(n))), at)
                .response
        );
    }
    assert_ok!(
        core.apply(SetNavigationTarget(Some(target(10))), at)
            .response
    );
    let recents = core.apply(GetRecentTargets, at).response;
    assert_eq!(recents.len(), 30);
    assert_eq!(recents[0], target(10));
    assert_eq!(recents[29], target(1));
    let pins = assert_ok!(core.apply(PinTarget(target(0)), at).response);
    assert_ok!(core.apply(UnpinTarget(pins[0].id), at).response);
    assert_eq!(core.apply(GetRecentTargets, at).response[0], target(0));
    assert_ok!(core.apply(UnpinTarget(pins[0].id), at).response);
    assert_eq!(core.apply(GetRecentTargets, at).response.len(), 30);
}

#[test]
fn invalid_history_does_not_replace_the_current_list() {
    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::default();
    let target = NavigationTarget::MapPosition {
        latitude_degrees: 50.,
        longitude_degrees: 6.,
    };
    assert_ok!(
        core.apply(SetNavigationTarget(Some(target.clone())), at)
            .response
    );
    claims::assert_err!(
        core.apply(
            updraft_core::RestoreRecentTargets(vec![NavigationTarget::MapPosition {
                latitude_degrees: 91.,
                longitude_degrees: 0.
            }]),
            at
        )
        .response
    );
    assert_eq!(core.apply(GetRecentTargets, at).response, vec![target]);
}
