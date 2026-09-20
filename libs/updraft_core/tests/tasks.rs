use claims::{assert_err, assert_ok};
use updraft_core::{
    ChangeTask, Core, GetTask, NavigationTarget, SettingsSnapshot, TaskCommand, TaskStatus,
    Timestamp,
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
    claims::assert_some_eq!(task.current, 1);
    assert_eq!(task.status, TaskStatus::Running);
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
    assert_eq!(core.apply(GetTask, at).response.status, TaskStatus::Stopped);
}

fn route() -> Core {
    let mut core = Core::new(SettingsSnapshot::default());
    for (name, longitude_degrees) in [("Start", 0.), ("Turn", 0.02), ("Finish", 0.04)] {
        assert_ok!(
            core.apply(
                ChangeTask(TaskCommand::Add {
                    target: NavigationTarget::Waypoint {
                        name: name.into(),
                        latitude_degrees: 0.,
                        longitude_degrees,
                        elevation_meters: 0.
                    }
                }),
                Timestamp::default()
            )
            .response
        );
    }
    assert_ok!(
        core.apply(
            ChangeTask(TaskCommand::Select { id: 0 }),
            Timestamp::default()
        )
        .response
    );
    core
}

fn fix(core: &mut Core, longitude: f64, millis: u64) {
    core.apply(
        updraft_core::InternalGps::new(updraft_core::Fix {
            position: updraft_geo::LatLon::from_degrees(0., longitude),
            altitude_ellipsoid: None,
            track: None,
            ground_speed: None,
            fix_time: Some(updraft_core::UtcInstant::from_unix_milliseconds(
                millis as i64,
            )),
        }),
        Timestamp::from_millis(millis),
    );
}

#[test]
fn task_advances_through_a_cylinder_between_reports_while_goto_remains_primary() {
    let mut core = route();
    fix(&mut core, 0., 1000);
    fix(&mut core, 0.006, 2000);
    claims::assert_some_eq!(
        core.apply(GetTask, Timestamp::from_millis(2000))
            .response
            .current,
        1
    );
    assert_ok!(
        core.apply(
            updraft_core::SetNavigationTarget(Some(waypoint("Elsewhere"))),
            Timestamp::from_millis(2000)
        )
        .response
    );
    fix(&mut core, 0.014, 3000);
    fix(&mut core, 0.026, 4000);
    claims::assert_some_eq!(
        core.apply(GetTask, Timestamp::from_millis(4000))
            .response
            .current,
        2
    );
    fix(&mut core, 0.04, 5000);
    assert_eq!(
        core.apply(GetTask, Timestamp::from_millis(5000))
            .response
            .status,
        TaskStatus::Completed
    );
    claims::assert_some_eq!(
        core.apply(
            updraft_core::GetNavigationTarget,
            Timestamp::from_millis(5000)
        )
        .response,
        waypoint("Elsewhere")
    );
}

#[test]
fn gap_limit_and_manual_selection_require_a_new_entry() {
    let mut core = route();
    assert_ok!(
        core.apply(
            ChangeTask(TaskCommand::Select { id: 1 }),
            Timestamp::default()
        )
        .response
    );
    fix(&mut core, 0.02, 1000);
    fix(&mut core, 0.021, 2000);
    claims::assert_some_eq!(
        core.apply(GetTask, Timestamp::from_millis(2000))
            .response
            .current,
        1
    );
    fix(&mut core, 0.014, 3000);
    fix(&mut core, 0.026, 13001);
    claims::assert_some_eq!(
        core.apply(GetTask, Timestamp::from_millis(13001))
            .response
            .current,
        1
    );
    fix(&mut core, 0.014, 23001);
    claims::assert_some_eq!(
        core.apply(GetTask, Timestamp::from_millis(23001))
            .response
            .current,
        2
    );
}

#[test]
fn restarts_close_after_a_turnpoint_and_reopen_after_explicit_start_selection() {
    let mut core = route();
    fix(&mut core, 0., 1000);
    fix(&mut core, 0.006, 2000);
    let first = core
        .apply(GetTask, Timestamp::from_millis(2000))
        .response
        .start;
    fix(&mut core, 0., 3000);
    fix(&mut core, 0.006, 4000);
    let restarted = core
        .apply(GetTask, Timestamp::from_millis(4000))
        .response
        .start;
    assert_ne!(first, restarted);
    fix(&mut core, 0.02, 5000);
    fix(&mut core, 0., 6000);
    fix(&mut core, -0.006, 7000);
    assert_eq!(
        core.apply(GetTask, Timestamp::from_millis(7000))
            .response
            .start,
        restarted
    );
    assert_ok!(
        core.apply(
            ChangeTask(TaskCommand::Select { id: 0 }),
            Timestamp::from_millis(7000)
        )
        .response
    );
    fix(&mut core, 0., 8000);
    fix(&mut core, 0.006, 9000);
    assert_ne!(
        core.apply(GetTask, Timestamp::from_millis(9000))
            .response
            .start,
        restarted
    );
}

#[test]
fn restoring_progress_and_selecting_after_completion_do_not_infer_crossings() {
    let mut core = route();
    fix(&mut core, 0., 1000);
    fix(&mut core, 0.006, 2000);
    let saved = core.apply(GetTask, Timestamp::from_millis(2000)).response;
    let mut restored = Core::new(SettingsSnapshot::default());
    assert_ok!(
        restored
            .apply(
                updraft_core::RestoreTask(saved.clone()),
                Timestamp::default()
            )
            .response
    );
    fix(&mut restored, 0.026, 1000);
    assert_eq!(
        restored
            .apply(GetTask, Timestamp::from_millis(1000))
            .response,
        saved
    );
    fix(&mut restored, 0.014, 2000);
    fix(&mut restored, 0.04, 3000);
    let finished = restored
        .apply(GetTask, Timestamp::from_millis(3000))
        .response;
    assert_eq!(finished.status, TaskStatus::Completed);
    assert_ok!(
        restored
            .apply(
                ChangeTask(TaskCommand::Select { id: 1 }),
                Timestamp::from_millis(3000)
            )
            .response
    );
    let resumed = restored
        .apply(GetTask, Timestamp::from_millis(3000))
        .response;
    assert_eq!(resumed.start, saved.start);
    claims::assert_none!(resumed.finish);
    assert_eq!(resumed.status, TaskStatus::Running);
}

#[test]
fn restoring_a_navigation_reference_does_not_resume_a_stopped_task() {
    let mut core = route();
    assert_ok!(
        core.apply(ChangeTask(TaskCommand::Stop), Timestamp::default())
            .response
    );
    assert_ok!(
        core.apply(
            updraft_core::RestoreNavigationTarget(Some(NavigationTarget::Task)),
            Timestamp::default()
        )
        .response
    );
    assert_eq!(
        core.apply(GetTask, Timestamp::default()).response.status,
        TaskStatus::Stopped
    );
    claims::assert_none!(
        core.apply(updraft_core::GetNavigationTarget, Timestamp::default())
            .response
    );
}

#[test]
fn live_edits_keep_point_identity_and_deletion_selects_a_neighbor_without_finishing() {
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
    claims::assert_some_eq!(core.apply(GetTask, at).response.current, 1);
    assert_ok!(
        core.apply(ChangeTask(TaskCommand::Remove { id: 1 }), at)
            .response
    );
    let task = core.apply(GetTask, at).response;
    claims::assert_some_eq!(task.current, 2);
    assert_eq!(task.status, TaskStatus::Running);
    claims::assert_none!(task.finish);
    assert_err!(
        core.apply(ChangeTask(TaskCommand::Remove { id: 2 }), at)
            .response
    );
    assert_eq!(core.apply(GetTask, at).response, task);
}

#[test]
fn skipping_closes_the_restart_window_and_finish_clears_only_task_guidance() {
    let mut core = route();
    let at = Timestamp::default();
    assert_ok!(
        core.apply(ChangeTask(TaskCommand::Select { id: 2 }), at)
            .response
    );
    fix(&mut core, 0., 1000);
    fix(&mut core, 0.006, 2000);
    claims::assert_none!(
        core.apply(GetTask, Timestamp::from_millis(2000))
            .response
            .start
    );
    fix(&mut core, 0.04, 3000);
    let task = core.apply(GetTask, Timestamp::from_millis(3000)).response;
    assert_eq!(task.status, TaskStatus::Completed);
    claims::assert_none!(
        core.apply(
            updraft_core::GetNavigationTarget,
            Timestamp::from_millis(3000)
        )
        .response
    );
}
