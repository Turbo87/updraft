use super::support::core_with_external_device;
use claims::assert_ok;
use updraft_core::{
    Bytes, ChangeTask, Effect, NavigationTarget, SetNavigationTarget, TaskCommand, Timestamp, Topic,
};

#[test]
fn replayed_nmea_positions_advance_the_task_while_a_goto_remains_selected() {
    let (mut core, device_id) = core_with_external_device();
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
    assert_ok!(
        core.apply(
            SetNavigationTarget(Some(NavigationTarget::Traffic {
                id: updraft_core::TrafficTargetId::new(
                    updraft_core::TrafficTargetIdType::Icao,
                    0xABC123
                )
            })),
            Timestamp::default()
        )
        .response
    );
    let mut progress = Vec::new();
    for (index, longitude_minutes) in [0., 0.36, 0.84, 1.56, 2.4].into_iter().enumerate() {
        let seconds = index + 1;
        let sentence = format!(
            "$GPRMC,1200{seconds:02}.00,A,0000.000,N,000{longitude_minutes:06.3},E,60.0,90.0,200926,,,A\r\n"
        );
        let update = core.apply(
            Bytes::new(device_id, sentence.into_bytes()),
            Timestamp::from_millis(seconds as u64 * 1000),
        );
        for effect in update.effects {
            if let Effect::Emit(Topic::Task(task)) = effect {
                progress.push(serde_json::json!({"current":task.current,"status":task.status,"start":task.start,"finish":task.finish,"restartAllowed":task.restart_allowed}));
            }
        }
    }
    assert_eq!(progress.len(), 3);
    insta::assert_json_snapshot!(progress);
    claims::assert_some_eq!(
        core.apply(
            updraft_core::GetNavigationTarget,
            Timestamp::from_millis(5000)
        )
        .response,
        NavigationTarget::Traffic {
            id: updraft_core::TrafficTargetId::new(
                updraft_core::TrafficTargetIdType::Icao,
                0xABC123
            )
        }
    );
}

#[test]
fn batched_position_reports_do_not_skip_task_crossings() {
    check_batched_positions(false, 43_200);
}

#[test]
fn batched_gga_reports_do_not_skip_task_crossings() {
    check_batched_positions(true, 43_200);
}

#[test]
fn batched_gga_reports_cross_midnight() {
    check_batched_positions(true, 86_398);
}

fn check_batched_positions(gga: bool, start_seconds: usize) {
    let (mut core, device_id) = core_with_external_device();
    for longitude_degrees in [0., 0.02, 0.04] {
        assert_ok!(
            core.apply(
                ChangeTask(TaskCommand::Add {
                    target: NavigationTarget::Waypoint {
                        name: "Point".into(),
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
    let data = [0., 0.36, 0.84, 1.56, 2.4]
        .into_iter()
        .enumerate()
        .map(|(index, minutes)| {
            if gga {
                let seconds = (start_seconds + index) % 86_400;
                let clock = format!(
                    "{:02}{:02}{:02}",
                    seconds / 3600,
                    seconds / 60 % 60,
                    seconds % 60
                );
                return format!(
                    "$GPGGA,{clock}.00,0000.000,N,000{minutes:06.3},E,1,08,1.0,1000.0,M,0.0,M,,\r\n"
                );
            }
            format!(
                "$GPRMC,1200{:02}.00,A,0000.000,N,000{minutes:06.3},E,60.0,90.0,200926,,,A\r\n",
                index + 1
            )
        })
        .collect::<String>();
    core.apply(
        Bytes::new(device_id, data.into_bytes()),
        Timestamp::from_millis(5000),
    );
    assert_eq!(
        core.apply(updraft_core::GetTask, Timestamp::from_millis(5000))
            .response
            .status,
        updraft_core::TaskStatus::Completed
    );
}
