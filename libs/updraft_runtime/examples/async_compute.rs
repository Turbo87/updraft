//! An end-to-end run of the async-computation path.
//!
//! Run with: `cargo run -p updraft_runtime --example async_compute`
//!
//! Samples are submitted on the main thread (the cheap, synchronous side).
//! A background timer batches them at a fixed cadence and hands each batch
//! to a worker, which reduces it off the input loop and returns the result
//! as a change. A printer thread renders the change stream so the two paths
//! are visible side by side: `[sync]` lines land immediately per sample,
//! `[async]` lines arrive from the worker a moment later.

use std::thread;
use std::time::Duration;

use updraft_core::Change;
use updraft_runtime::{Runtime, RuntimeConfig};

fn main() {
    let runtime = Runtime::with_config(RuntimeConfig {
        // A brisk ~4 Hz cadence and a 60 ms "expensive" job, so the demo
        // finishes in a couple of seconds.
        interval_millis: 250,
        worker_delay: Duration::from_millis(60),
        ..Default::default()
    });

    let subscription = runtime.subscribe();
    println!("subscribed; initial snapshot = {:?}", subscription.snapshot);

    let changes = subscription.changes;
    let printer = thread::spawn(move || {
        while let Ok(batch) = changes.recv() {
            for change in batch {
                match change {
                    Change::Samples(count) => {
                        println!("  [sync]  observed sample, count now {count}");
                    }
                    Change::Computed(reduction) => {
                        println!(
                            "  [async] worker reduced {} samples -> sum {}",
                            reduction.count, reduction.sum,
                        );
                    }
                }
            }
        }
    });

    println!("feeding 20 samples over ~1.5s...");
    for value in 1..=20 {
        runtime.submit_observe(value);
        thread::sleep(Duration::from_millis(75));
    }

    // Let the final batch finish on the worker before tearing down.
    thread::sleep(Duration::from_millis(400));
    runtime.shutdown();

    printer.join().ok();
    println!("done");
}
