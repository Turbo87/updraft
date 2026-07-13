use std::time::{Duration, Instant};

use updraft_core::flight::{
    Change as FlightChange, GetPosition, MslAltitude, Observation as FlightObservation, PositionFix,
};
use updraft_core::{App, AppConfig, Change, Input};
use updraft_geo::LatLon;
use updraft_runtime::{ChangeFilter, Handle, Runtime, RuntimeStopped};
use updraft_units::Length;

const TIMEOUT: Duration = Duration::from_secs(10);

fn app() -> App {
    App::with_config(AppConfig {
        flight: updraft_core::flight::Config {
            trace_stats_interval: Duration::from_millis(10),
        },
    })
}

fn fix(handle: &Handle, latitude: f64) -> PositionFix {
    PositionFix {
        observed_at: handle.clock_time(),
        position: LatLon::from_degrees(latitude, 6.),
        altitude: Some(MslAltitude::new(Length::from_meters(1_000.))),
        track: None,
        ground_speed: None,
    }
}

fn submit_fix(handle: &Handle, latitude: f64) -> PositionFix {
    let fix = fix(handle, latitude);
    handle
        .submit(Input::Flight(updraft_core::flight::Input::Observation(
            FlightObservation::Position(fix),
        )))
        .unwrap();
    fix
}

#[test]
fn atomic_subscribe_and_fifo_ordering() {
    let runtime = Runtime::builder(app()).start();
    let handle = runtime.handle();
    let first = submit_fix(&handle, 50.);

    let subscription = handle.subscribe(ChangeFilter::all()).unwrap();
    assert_eq!(subscription.snapshot.flight.position, Some(first));

    let second = submit_fix(&handle, 51.);
    let third = submit_fix(&handle, 52.);
    let mut positions = Vec::new();
    let deadline = Instant::now() + TIMEOUT;
    while positions.len() < 2 {
        let changes = subscription
            .changes
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        positions.extend(changes.into_iter().filter_map(|change| match change {
            Change::Flight(FlightChange::Position(fix)) => Some(fix),
            Change::Flight(FlightChange::TraceStats(_)) => None,
        }));
    }
    assert_eq!(positions, vec![second, third]);
    runtime.shutdown();
}

#[test]
fn subscription_omits_unselected_change_groups() {
    let runtime = Runtime::builder(app()).start();
    let handle = runtime.handle();
    let subscription = handle.subscribe(ChangeFilter::only([])).unwrap();

    let fix = submit_fix(&handle, 50.);
    assert_eq!(handle.query(GetPosition).unwrap(), Some(fix));

    assert_eq!(
        subscription.changes.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    );
    assert_eq!(handle.metrics().slow_subscriber_drops(), 0);
    runtime.shutdown();
}

#[test]
fn slow_subscriber_is_dropped() {
    let runtime = Runtime::builder(app())
        .subscriber_buffer_capacity(1)
        .start();
    let handle = runtime.handle();
    let subscription = handle.subscribe(ChangeFilter::all()).unwrap();
    for i in 0..10 {
        submit_fix(&handle, 50. + f64::from(i));
    }
    handle.query(GetPosition).unwrap();

    assert_eq!(handle.metrics().slow_subscriber_drops(), 1);
    let _ = subscription.changes.recv_timeout(TIMEOUT).unwrap();
    assert_eq!(subscription.changes.recv(), Err(std::sync::mpsc::RecvError));
    runtime.shutdown();
}

#[test]
fn runtime_records_queue_and_handler_measurements() {
    let runtime = Runtime::builder(app()).start();
    let handle = runtime.handle();

    submit_fix(&handle, 50.);
    handle
        .query(GetPosition)
        .expect("query orders measurement after the input");

    assert!(handle.metrics().max_pending_messages() >= 1);
    assert!(handle.metrics().queue_wait_samples() >= 2);
    assert!(handle.metrics().inputs_handled() >= 1);
    assert!(
        handle.metrics().total_handler_time() >= handle.metrics().max_handler_time(),
        "the maximum handler duration is part of the total"
    );
    runtime.shutdown();
}

#[test]
fn missing_worker_fails_the_job_without_stalling() {
    let runtime = Runtime::builder(app()).start();
    let handle = runtime.handle();

    submit_fix(&handle, 50.);

    let deadline = Instant::now() + TIMEOUT;
    while handle.metrics().worker_failures() == 0 {
        assert!(Instant::now() < deadline, "job failure was never recorded");
        std::thread::yield_now();
    }

    assert!(handle.query(GetPosition).unwrap().is_some());
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
    assert!(handle.subscribe(ChangeFilter::all()).is_err());
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
