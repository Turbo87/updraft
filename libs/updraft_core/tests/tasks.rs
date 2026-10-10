use claims::{assert_err, assert_none, assert_ok, assert_some_eq};
use updraft_core::{
    ChangeTask, Core, GetNavigationTarget, GetRecentTargets, GetTask, NavigationTarget,
    PublishedTask, SetNavigationTarget, SettingsSnapshot, TaskCommand, TaskProgress, TaskTime,
    Timestamp, TrafficTargetId, TrafficTargetIdType,
};

fn waypoint(name: &str) -> NavigationTarget {
    NavigationTarget::Waypoint {
        name: name.into(),
        latitude_degrees: 50.,
        longitude_degrees: 6.,
        elevation_meters: 100.,
    }
}

#[test]
fn a_task_can_be_edited_selected_and_pinned_independently_of_goto() {
    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::default();
    assert_ok!(
        core.apply(
            ChangeTask(TaskCommand::Add {
                target: waypoint("Start")
            }),
            at
        )
        .response
    );
    assert_err!(
        core.apply(ChangeTask(TaskCommand::Select { id: 0 }), at)
            .response
    );
    assert_ok!(
        core.apply(
            ChangeTask(TaskCommand::Add {
                target: waypoint("Finish")
            }),
            at
        )
        .response
    );
    assert_ok!(
        core.apply(ChangeTask(TaskCommand::Select { id: 1 }), at)
            .response
    );
    let task = core.apply(GetTask, at).response;
    assert_some_eq!(task.target, 1);
    assert_ok!(
        core.apply(updraft_core::PinTarget(NavigationTarget::Task), at)
            .response
    );
    assert_ok!(
        core.apply(
            updraft_core::SetNavigationTarget(Some(waypoint("Elsewhere"))),
            at
        )
        .response
    );
    assert_eq!(core.apply(GetTask, at).response, task);
    assert_ok!(core.apply(ChangeTask(TaskCommand::Stop), at).response);
    assert_eq!(core.apply(GetTask, at).response, PublishedTask::default());
}

/// Adds waypoints at the given latitudes and longitudes as a task.
fn route_through(points: &[(f64, f64)]) -> Core {
    let mut core = Core::new(SettingsSnapshot::default());
    for &(latitude_degrees, longitude_degrees) in points {
        let target = NavigationTarget::Waypoint {
            name: "Point".into(),
            latitude_degrees,
            longitude_degrees,
            elevation_meters: 0.,
        };
        let add = ChangeTask(TaskCommand::Add { target });
        assert_ok!(core.apply(add, Timestamp::default()).response);
    }
    core
}

/// A start, a turnpoint, and a finish 2.2 km apart along the equator.
fn route() -> Core {
    route_through(&[(0., 0.), (0., 0.02), (0., 0.04)])
}

fn task(core: &mut Core) -> PublishedTask {
    core.apply(GetTask, Timestamp::default()).response
}

fn time(unix_milliseconds: i64) -> Option<TaskTime> {
    Some(TaskTime { unix_milliseconds })
}

fn fix(core: &mut Core, longitude: f64, millis: u64) {
    position_fix(core, 0., longitude, millis);
}

fn position_fix(core: &mut Core, latitude: f64, longitude: f64, millis: u64) {
    fix_with_utc(core, latitude, longitude, Some(millis as i64), millis);
}

fn fix_with_utc(core: &mut Core, latitude: f64, longitude: f64, utc: Option<i64>, millis: u64) {
    core.apply(
        updraft_core::InternalGps::new(updraft_core::Fix {
            position: updraft_geo::LatLon::from_degrees(latitude, longitude),
            altitude_ellipsoid: None,
            track: None,
            ground_speed: None,
            fix_time: utc.map(updraft_core::UtcInstant::from_unix_milliseconds),
        }),
        Timestamp::from_millis(millis),
    );
}

#[test]
fn task_advances_through_a_cylinder_between_reports_while_goto_remains_primary() {
    let mut core = route();
    fix(&mut core, 0., 1000);
    fix(&mut core, 0.006, 2000);
    assert_some_eq!(task(&mut core).target, 1);
    assert_ok!(
        core.apply(
            updraft_core::SetNavigationTarget(Some(waypoint("Elsewhere"))),
            Timestamp::from_millis(2000)
        )
        .response
    );
    fix(&mut core, 0.014, 3000);
    fix(&mut core, 0.026, 4000);
    assert_some_eq!(task(&mut core).target, 2);
    fix(&mut core, 0.04, 5000);
    let task = task(&mut core);
    assert_none!(task.target);
    assert_eq!(
        task.progress,
        TaskProgress {
            reached: 3,
            start: time(1749),
            finish: time(4679),
        }
    );
    assert_some_eq!(
        core.apply(GetNavigationTarget, Timestamp::from_millis(5000))
            .response,
        waypoint("Elsewhere")
    );
}

#[test]
fn the_start_is_the_last_start_exit_before_the_second_point() {
    let mut core = route();
    fix(&mut core, 0., 1000);
    fix(&mut core, 0.006, 2000);
    fix(&mut core, 0., 3000);
    fix(&mut core, 0.006, 4000);
    assert_eq!(task(&mut core).progress.start, time(3749));
    fix(&mut core, 0.02, 5000);
    fix(&mut core, 0., 6000);
    fix(&mut core, -0.006, 7000);
    let progress = task(&mut core).progress;
    assert_eq!(progress.reached, 2);
    assert_eq!(progress.start, time(3749));
}

#[test]
fn the_second_point_of_a_two_point_task_is_the_finish() {
    let mut core = route_through(&[(0., 0.), (0., 0.02)]);
    fix(&mut core, 0., 1000);
    fix(&mut core, 0.02, 2000);
    fix(&mut core, 0., 3000);
    fix(&mut core, -0.006, 4000);
    assert_eq!(
        task(&mut core).progress,
        TaskProgress {
            reached: 2,
            start: time(1225),
            finish: time(1775),
        }
    );
}

#[test]
fn points_count_only_in_route_order_and_the_first_finish_entry_counts() {
    let mut core = route_through(&[(0., 0.), (0.02, 0.02), (0., 0.04)]);
    position_fix(&mut core, 0., 0.04, 1000);
    position_fix(&mut core, 0., 0., 2000);
    position_fix(&mut core, 0., 0.006, 3000);
    position_fix(&mut core, 0., 0.04, 4000);
    let progress = task(&mut core).progress;
    assert_eq!(progress.reached, 1);
    assert_none!(progress.finish);
    position_fix(&mut core, 0.02, 0.02, 5000);
    position_fix(&mut core, 0., 0.04, 6000);
    position_fix(&mut core, 0., 0.03, 7000);
    position_fix(&mut core, 0., 0.04, 8000);
    assert_eq!(
        task(&mut core).progress,
        TaskProgress {
            reached: 3,
            start: time(2749),
            finish: time(5841),
        }
    );
}

#[test]
fn a_crossing_counts_after_a_gap_longer_than_10_seconds() {
    let mut core = route();
    fix(&mut core, 0., 1000);
    fix(&mut core, 0.006, 61_000);
    assert_eq!(task(&mut core).progress.start, time(45_916));
}

#[test]
fn fixes_that_the_recorder_does_not_record_do_not_count() {
    let mut core = route();
    fix(&mut core, 0., 1000);
    // The fix time of a source is fresh for 3 s, so the fix at 5 s has no UTC.
    fix_with_utc(&mut core, 0., 0.006, None, 5000);
    fix(&mut core, 0., 6000);
    fix_with_utc(&mut core, 0., 0.006, Some(1000), 7000);
    fix(&mut core, 0., 8000);
    assert_eq!(task(&mut core).progress, TaskProgress::default());
}

#[test]
fn manual_selection_does_not_change_progress() {
    let mut core = route();
    fix(&mut core, 0., 1000);
    fix(&mut core, 0.006, 2000);
    fix(&mut core, 0.02, 3000);
    let progress = task(&mut core).progress;
    let select = ChangeTask(TaskCommand::Select { id: 0 });
    assert_ok!(core.apply(select, Timestamp::from_millis(3000)).response);
    assert_eq!(task(&mut core).progress, progress);
    fix(&mut core, 0., 4000);
    fix(&mut core, 0.006, 5000);
    let task = task(&mut core);
    assert_some_eq!(task.target, 1);
    assert_eq!(task.progress, progress);
}

#[test]
fn a_selected_target_needs_a_new_entry() {
    let mut core = route();
    let select = ChangeTask(TaskCommand::Select { id: 1 });
    assert_ok!(core.apply(select, Timestamp::default()).response);
    fix(&mut core, 0.02, 1000);
    fix(&mut core, 0.021, 2000);
    assert_some_eq!(task(&mut core).target, 1);
    fix(&mut core, 0.014, 3000);
    fix(&mut core, 0.026, 4000);
    assert_some_eq!(task(&mut core).target, 2);
}

#[test]
fn a_start_exit_moves_the_target_from_the_start_to_the_second_point() {
    let mut core = route();
    fix(&mut core, 0.02, 1000);
    fix(&mut core, 0., 2000);
    assert_some_eq!(task(&mut core).target, 0);
    fix(&mut core, -0.006, 3000);
    assert_some_eq!(task(&mut core).target, 1);
}

#[test]
fn the_target_follows_a_skipped_point_and_clears_task_navigation_at_the_finish() {
    let mut core = route_through(&[(0., 0.), (0.02, 0.02), (0., 0.04), (0., 0.06)]);
    fix(&mut core, 0., 1000);
    fix(&mut core, 0.006, 2000);
    assert_some_eq!(task(&mut core).target, 1);
    let select = ChangeTask(TaskCommand::Select { id: 2 });
    assert_ok!(core.apply(select, Timestamp::from_millis(2000)).response);
    fix(&mut core, 0.04, 3000);
    let task_at_tp3 = task(&mut core);
    assert_some_eq!(task_at_tp3.target, 3);
    assert_eq!(task_at_tp3.progress.reached, 1);
    fix(&mut core, 0.06, 4000);
    let finished = task(&mut core);
    assert_none!(finished.target);
    assert_eq!(finished.progress.reached, 1);
    assert_none!(finished.progress.finish);
    assert_none!(
        core.apply(GetNavigationTarget, Timestamp::from_millis(4000))
            .response
    );
}

#[test]
fn a_route_edit_derives_progress_again_from_the_recorded_fixes() {
    let mut core = route_through(&[(0., 0.), (0., 0.04)]);
    fix(&mut core, 0., 1000);
    fix(&mut core, 0.006, 2000);
    fix(&mut core, 0.026, 3000);
    let target = NavigationTarget::Waypoint {
        name: "Turn".into(),
        latitude_degrees: 0.,
        longitude_degrees: 0.02,
        elevation_meters: 0.,
    };
    let at = Timestamp::from_millis(3000);
    assert_ok!(
        core.apply(ChangeTask(TaskCommand::Add { target }), at)
            .response
    );
    let move_up = ChangeTask(TaskCommand::Move { id: 2, index: 1 });
    assert_ok!(core.apply(move_up, at).response);
    let task = task(&mut core);
    assert_some_eq!(task.target, 1);
    assert_eq!(
        task.progress,
        TaskProgress {
            reached: 2,
            start: time(1749),
            finish: None,
        }
    );
}

#[test]
fn a_route_edit_targets_the_first_unreached_point() {
    let mut core = route();
    let at = Timestamp::default();
    assert_ok!(
        core.apply(ChangeTask(TaskCommand::Select { id: 1 }), at)
            .response
    );
    assert_ok!(
        core.apply(ChangeTask(TaskCommand::Move { id: 1, index: 2 }), at)
            .response
    );
    assert_some_eq!(task(&mut core).target, 0);
    assert_ok!(
        core.apply(ChangeTask(TaskCommand::Remove { id: 1 }), at)
            .response
    );
    assert_some_eq!(task(&mut core).target, 0);
    assert_ok!(
        core.apply(ChangeTask(TaskCommand::Remove { id: 2 }), at)
            .response
    );
    let task = task(&mut core);
    assert_eq!(task.route.points.len(), 1);
    assert_none!(task.target);
}

#[test]
fn restoring_a_route_targets_its_start() {
    let route = task(&mut route()).route;
    let mut core = Core::new(SettingsSnapshot::default());
    let restore = updraft_core::RestoreTask(route.clone());
    assert_ok!(core.apply(restore, Timestamp::default()).response);
    assert_eq!(
        task(&mut core),
        PublishedTask {
            route,
            target: Some(0),
            progress: TaskProgress::default(),
        }
    );
}

#[test]
fn restoring_task_navigation_without_a_task_target_selects_nothing() {
    let mut core = Core::new(SettingsSnapshot::default());
    assert_ok!(
        core.apply(
            updraft_core::RestoreNavigationTarget(Some(NavigationTarget::Task)),
            Timestamp::default()
        )
        .response
    );
    assert_none!(
        core.apply(GetNavigationTarget, Timestamp::default())
            .response
    );
}

#[test]
fn selecting_a_moved_point_navigates_to_the_task_without_a_recent_target() {
    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::default();
    for name in ["Start", "Turn", "Finish"] {
        let target = waypoint(name);
        let add = ChangeTask(TaskCommand::Add { target });
        assert_ok!(core.apply(add, at).response);
    }
    let move_up = ChangeTask(TaskCommand::Move { id: 2, index: 1 });
    assert_ok!(core.apply(move_up, at).response);
    let task = core.apply(GetTask, at).response;
    let ids = task
        .route
        .points
        .iter()
        .map(|point| point.id)
        .collect::<Vec<_>>();
    assert_eq!(ids, [0, 2, 1]);
    let select = ChangeTask(TaskCommand::Select { id: 2 });
    assert_ok!(core.apply(select, at).response);
    assert_some_eq!(core.apply(GetTask, at).response.target, 2);
    let navigation = core.apply(GetNavigationTarget, at).response;
    assert_some_eq!(navigation, NavigationTarget::Task);
    assert_eq!(core.apply(GetRecentTargets, at).response, vec![]);
}

#[test]
fn stopping_a_task_clears_only_task_navigation() {
    let mut core = route();
    let at = Timestamp::default();
    let select = ChangeTask(TaskCommand::Select { id: 0 });
    assert_ok!(core.apply(select, at).response);
    assert_ok!(core.apply(ChangeTask(TaskCommand::Stop), at).response);
    assert_none!(core.apply(GetNavigationTarget, at).response);
    let mut core = route();
    let traffic = NavigationTarget::Traffic {
        id: TrafficTargetId::new(TrafficTargetIdType::Icao, 0xABC123),
    };
    let goto = SetNavigationTarget(Some(traffic.clone()));
    assert_ok!(core.apply(goto, at).response);
    assert_ok!(core.apply(ChangeTask(TaskCommand::Stop), at).response);
    assert_some_eq!(core.apply(GetNavigationTarget, at).response, traffic);
}
