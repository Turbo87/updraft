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
    assert_eq!(task.current, Some(1));
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
