//! Demonstrates the asynchronous compute path end to end.
//!
//! A fake positioning source feeds fixes into the runtime at 1 Hz. Each
//! fix updates the shared position state synchronously, while the core
//! requests trace statistics through `Effect::Compute`: the runtime hands
//! the job to a worker thread, and the result returns to the core as an
//! ordinary input before it is published as a change. Job starts are
//! throttled by a core timer to one every five seconds, and clearing the
//! trace mid-flight bumps the epoch so an in-flight result is discarded
//! instead of applied.
//!
//! Run with:
//!
//! ```text
//! cargo run -p updraft_runtime --example async_compute
//! ```

use std::thread;
use std::time::Duration;

use updraft_core::{App, Change, Command, ComputeKind, Input, Observation, PositionFix};
use updraft_geo::LatLon;
use updraft_runtime::{Handle, PureWorker, Runtime};
use updraft_units::{Angle, Length, Speed};

fn main() {
    let runtime = Runtime::builder(App::new())
        .worker(ComputeKind::TraceStats, PureWorker)
        .start();
    let handle = runtime.handle();

    // A fake positioning source: it plays the role of a device adapter,
    // stamping each observation with the runtime's process-wide clock.
    let source = thread::spawn({
        let handle = handle.clone();
        move || {
            fly_circle(&handle, 8);

            // Clearing the trace bumps the compute epoch: a result from a
            // job started before this command is rejected by the core.
            handle
                .submit(Input::Command(Command::ClearTrace))
                .expect("runtime is running");
            println!("→ trace cleared; in-flight results are now stale");

            fly_circle(&handle, 5);
        }
    });

    // A state-stream client: snapshot first, then change batches in
    // input order.
    let subscription = handle.subscribe().expect("runtime is running");
    println!("snapshot: {:?}", subscription.snapshot);
    while let Ok(changes) = subscription.changes.recv_timeout(Duration::from_secs(3)) {
        for change in changes {
            match change {
                Change::Position(fix) => {
                    println!(
                        "position: {:7.4}° {:7.4}° at {:6.1?}",
                        fix.position.latitude().as_degrees(),
                        fix.position.longitude().as_degrees(),
                        fix.altitude.unwrap_or(Length::ZERO),
                    );
                }
                Change::TraceStats(Some(stats)) => {
                    println!(
                        "trace stats (from the worker): {} fixes, {:.2} km flown, max {:.0?}",
                        stats.fix_count,
                        stats.distance.as_kilometers(),
                        stats.max_altitude.unwrap_or(Length::ZERO),
                    );
                }
                Change::TraceStats(None) => println!("trace stats reset"),
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
            observed_at: handle.now(),
            position: LatLon::from_degrees(50.75 + 0.01 * angle.cos(), 6.15 + 0.01 * angle.sin()),
            altitude: Some(Length::from_meters(1000. + f64::from(step) * 2.)),
            track: Some(Angle::from_radians(angle)),
            ground_speed: Some(Speed::from_kilometers_per_hour(95.)),
        };
        handle
            .submit(Input::Observation(Observation::Position(fix)))
            .expect("runtime is running");
        thread::sleep(Duration::from_secs(1));
    }
}
