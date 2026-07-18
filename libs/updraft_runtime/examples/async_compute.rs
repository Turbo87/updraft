//! Demonstrates trace statistics running on a compute worker.
//!
//! A fake positioning source feeds fixes into the runtime at 1 Hz.
//! Position changes are published immediately, while trace statistics
//! are calculated outside the core input loop.
//!
//! Run with:
//!
//! ```text
//! cargo run -p updraft_runtime --example async_compute
//! ```

use std::thread;
use std::time::Duration;

use updraft_core::flight::{
    Change as FlightChange, Command as FlightCommand, ComputeKind as FlightComputeKind,
    Observation as FlightObservation, PositionFix,
};
use updraft_core::{App, Change, ComputeKind, Input};
use updraft_geo::LatLon;
use updraft_runtime::{ChangeFilter, Handle, PureWorker, Runtime};
use updraft_units::{Angle, Length, MslAltitude, Speed};

fn main() {
    let runtime = Runtime::builder(App::new())
        .worker(
            ComputeKind::Flight(FlightComputeKind::TraceStats),
            PureWorker,
        )
        .start();
    let handle = runtime.handle();

    // A fake positioning source: it plays the role of a device adapter,
    // stamping each observation with the runtime clock.
    let source = thread::spawn({
        let handle = handle.clone();
        move || {
            fly_circle(&handle, 8);

            handle
                .submit(Input::Flight(updraft_core::flight::Input::Command(
                    FlightCommand::ClearTrace,
                )))
                .expect("runtime is running");
            println!("→ trace cleared");

            fly_circle(&handle, 5);
        }
    });

    // A state-stream client: snapshot first, then change batches in
    // input order.
    let subscription = handle
        .subscribe(ChangeFilter::all())
        .expect("runtime is running");
    println!("snapshot: {:?}", subscription.snapshot);
    while let Ok(changes) = subscription.changes.recv_timeout(Duration::from_secs(3)) {
        for change in changes {
            match change {
                Change::Flight(FlightChange::Position(fix)) => {
                    println!(
                        "position: {:7.4}° {:7.4}° at {:6.1?}",
                        fix.position.latitude().as_degrees(),
                        fix.position.longitude().as_degrees(),
                        fix.altitude.unwrap_or(MslAltitude::ZERO).length(),
                    );
                }
                Change::Flight(FlightChange::TraceStats(Some(stats))) => {
                    println!(
                        "trace stats (from the worker): {} fixes, {:.2} km flown, max {:.0?}",
                        stats.fix_count,
                        stats.distance.as_kilometers(),
                        stats.max_altitude.unwrap_or(MslAltitude::ZERO).length(),
                    );
                }
                Change::Flight(FlightChange::TraceStats(None)) => {
                    println!("trace stats reset");
                }
            }
        }
    }

    source.join().expect("source thread panicked");
    let metrics = handle.metrics();
    println!(
        "handled {} inputs, {} worker failures, {} slow-subscriber drops",
        metrics.inputs_handled(),
        metrics.worker_failures(),
        metrics.slow_subscriber_drops(),
    );
    runtime.shutdown();
}

/// Feeds `count` fixes along a circle near Aachen at 1 Hz.
fn fly_circle(handle: &Handle, count: u32) {
    for step in 0..count {
        let angle = f64::from(step) * 12f64.to_radians();
        let fix = PositionFix {
            observed_at: handle.clock_time(),
            position: LatLon::from_degrees(50.75 + 0.01 * angle.cos(), 6.15 + 0.01 * angle.sin()),
            altitude: Some(MslAltitude::new(Length::from_meters(
                1000. + f64::from(step) * 2.,
            ))),
            track: Some(Angle::from_radians(angle)),
            ground_speed: Some(Speed::from_kilometers_per_hour(95.)),
        };
        handle
            .submit(Input::Flight(updraft_core::flight::Input::Observation(
                FlightObservation::Position(fix),
            )))
            .expect("runtime is running");
        thread::sleep(Duration::from_secs(1));
    }
}
