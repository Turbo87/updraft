use super::super::*;
use super::support::*;
use crate::{NavigationTarget, SetNavigationTarget};
use claims::{assert_none, assert_ok, assert_some};

#[test]
fn traffic_navigation_follows_reports_retains_loss_and_waits_after_restart() {
    let (mut core, device) = core_with_external_device();
    let target = NavigationTarget::Traffic {
        id: assert_ok!(TrafficTargetId::try_from("icao:ABC123".to_owned())),
    };
    assert_ok!(
        core.apply(SetNavigationTarget(Some(target.clone())), at(0))
            .response
    );
    assert_ok!(core.apply(crate::PinTarget(target.clone()), at(0)).response);
    assert_none!(assert_some!(core.navigation()).position);
    core.apply(Bytes::new(device, RMC), at(0));
    core.apply(Bytes::new(device, PFLAA_A), at(0));
    assert_none!(assert_some!(assert_some!(core.navigation()).traffic).relative_altitude);
    core.apply(Bytes::new(device, GGA), at(0));
    core.apply(Bytes::new(device, PFLAA_A), at(1));
    let first = assert_some!(core.navigation());
    assert_eq!(core.pinned_targets()[0].navigation, first);
    assert_some!(first.guidance);
    let traffic = assert_some!(first.traffic);
    assert_eq!(assert_some!(traffic.relative_altitude).meters, 50.);
    core.apply(Bytes::new(device, PFLAA_A_REPLACEMENT), at(1000));
    let moved = assert_some!(core.navigation());
    assert_eq!(core.pinned_targets()[0].navigation, moved);
    assert_ne!(first.position, moved.position);
    core.apply(Tick, at(32000));
    assert!(traffic_snapshot(&core).is_empty());
    let lost = assert_some!(core.navigation());
    assert_eq!(core.pinned_targets()[0].navigation, lost);
    assert_eq!(lost.position, moved.position);
    let traffic = assert_some!(lost.traffic);
    assert!(traffic.stale);
    assert_eq!(traffic.age_seconds, 31);
    assert!(assert_some!(lost.guidance).stale);
    assert_ok!(core.apply(SetNavigationTarget(None), at(32000)).response);
    assert_ok!(
        core.apply(SetNavigationTarget(Some(target.clone())), at(32000))
            .response
    );
    assert_eq!(
        core.navigation(),
        Some(core.pinned_targets()[0].navigation.clone())
    );
    core.apply(Bytes::new(device, RMC), at(33000));
    core.apply(Bytes::new(device, PFLAA_A), at(33000));
    assert!(!assert_some!(assert_some!(core.navigation()).traffic).stale);
    let pins = assert_ok!(
        core.apply(crate::PinTarget(target.clone()), at(33000))
            .response
    );
    let mut restarted = Core::new(SettingsSnapshot::default());
    assert_ok!(
        restarted
            .apply(crate::RestorePinnedTargets(pins), at(0))
            .response
    );
    assert_ok!(
        restarted
            .apply(SetNavigationTarget(Some(target)), at(0))
            .response
    );
    let waiting = assert_some!(restarted.navigation());
    assert_eq!(restarted.pinned_targets()[0].navigation, waiting);
    assert_none!(waiting.position);
    assert_none!(waiting.guidance);
    assert_none!(waiting.traffic);
}
