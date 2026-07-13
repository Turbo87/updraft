use std::time::Duration;

use updraft_core::flight::{GetPosition, MslAltitude, Observation, PositionFix};
use updraft_core::{App, AppConfig, Input};
use updraft_geo::LatLon;
use updraft_runtime::{Handle, Runtime, RuntimeStopped};
use updraft_units::Length;

fn app() -> App {
    App::with_config(AppConfig {
        flight: updraft_core::flight::Config {
            trace_stats_interval: Duration::from_millis(10),
        },
    })
}

fn submit_fix(handle: &Handle, latitude: f64) -> PositionFix {
    let fix = PositionFix {
        observed_at: handle.clock_time(),
        position: LatLon::from_degrees(latitude, 6.),
        altitude: Some(MslAltitude::new(Length::from_meters(1_000.))),
        track: None,
        ground_speed: None,
    };
    handle
        .submit(Input::Flight(updraft_core::flight::Input::Observation(
            Observation::Position(fix),
        )))
        .unwrap();
    fix
}

#[test]
fn inputs_and_queries_share_fifo_order() {
    let runtime = Runtime::builder(app()).start();
    let handle = runtime.handle();
    let first = submit_fix(&handle, 50.);
    let second = submit_fix(&handle, 51.);

    assert_ne!(first, second);
    assert_eq!(handle.query(GetPosition).unwrap(), Some(second));
    runtime.shutdown();
}

#[test]
fn missing_worker_failure_does_not_stop_the_runtime() {
    let runtime = Runtime::builder(app()).start();
    let handle = runtime.handle();
    submit_fix(&handle, 50.);
    let second = submit_fix(&handle, 51.);

    assert_eq!(handle.query(GetPosition).unwrap(), Some(second));
    runtime.shutdown();
}

#[test]
fn handle_reports_runtime_stopped_after_shutdown() {
    let runtime = Runtime::builder(app()).start();
    let handle = runtime.handle();
    runtime.shutdown();

    let input = Input::Clock {
        clock_time: handle.clock_time(),
    };
    assert!(handle.submit(input).is_err());
    assert_eq!(handle.query(GetPosition), Err(RuntimeStopped));
}

#[test]
fn dropping_the_runtime_stops_the_core() {
    let runtime = Runtime::builder(app()).start();
    let handle = runtime.handle();
    submit_fix(&handle, 50.);

    drop(runtime);

    let input = Input::Clock {
        clock_time: handle.clock_time(),
    };
    assert!(handle.submit(input).is_err());
}
