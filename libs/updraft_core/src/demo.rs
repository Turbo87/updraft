//! The demo domain: the core-side state and job slot for the async worker.
//!
//! This is the example's stand-in for a real expensive calculation. It
//! owns no threads and does no work itself; it accumulates samples (the
//! cheap, synchronous side), decides *when* a worker job should run, and
//! stores the last reduction the worker returned.
//!
//! The slot enforces one outstanding job at a time. Samples that arrive
//! while a job runs set the `dirty` flag; when the job returns, the core
//! immediately launches a fresh one if more work accumulated. The trigger
//! *cadence* lives here, not in the runtime: this domain schedules its own
//! [`TimerId::Demo`](crate::time::TimerId::Demo) timer, so a future second
//! worker type could run on a different cadence without touching the
//! runtime.

use crate::compute::{ComputeJob, Reduction};

/// The core-side demo state: the accumulated samples, the job slot, and the
/// last accepted reduction.
#[derive(Clone, Debug)]
pub struct Demo {
    interval_millis: u64,
    samples: Vec<u32>,
    dirty: bool,
    running: bool,
    latest: Option<Reduction>,
}

impl Demo {
    /// Creates a domain that re-evaluates at most once per `interval_millis`.
    pub fn new(interval_millis: u64) -> Self {
        Self {
            interval_millis,
            samples: Vec::new(),
            dirty: false,
            running: false,
            latest: None,
        }
    }

    /// The re-evaluation cadence, in milliseconds.
    pub fn interval_millis(&self) -> u64 {
        self.interval_millis
    }

    /// Records a sample, returning the new sample count. Marks the domain
    /// dirty so the next timer tick launches a job.
    pub fn record(&mut self, value: u32) -> usize {
        self.samples.push(value);
        self.dirty = true;
        self.samples.len()
    }

    /// The number of samples accumulated so far.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// The most recently accepted reduction, if any.
    pub fn latest(&self) -> Option<Reduction> {
        self.latest
    }

    /// Whether samples have arrived that have not yet been reduced.
    pub fn has_pending_work(&self) -> bool {
        self.dirty
    }

    /// Whether work is pending and no job is running, i.e. a job could
    /// start right now.
    pub fn can_start(&self) -> bool {
        self.dirty && !self.running
    }

    /// Whether a job is currently outstanding on the worker.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Launches a job if one is warranted, returning it for the caller to
    /// emit as an effect.
    ///
    /// A job starts only when work is pending and the slot is idle.
    /// Starting one snapshots the current samples into the job, clears the
    /// dirty flag, and marks the slot running.
    #[must_use]
    pub fn start_job(&mut self) -> Option<ComputeJob> {
        if !self.can_start() {
            return None;
        }
        self.dirty = false;
        self.running = true;
        Some(ComputeJob::Demo {
            batch: self.samples.clone(),
        })
    }

    /// Accepts a completed reduction, freeing the slot and storing the
    /// result.
    pub fn accept(&mut self, reduction: Reduction) {
        self.running = false;
        self.latest = Some(reduction);
    }

    /// Records that the worker failed, freeing the slot so a later job can
    /// run. The previously accepted reduction is kept.
    pub fn record_failure(&mut self) {
        self.running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_most_one_job() {
        let mut demo = Demo::new(1000);
        assert!(demo.start_job().is_none(), "nothing recorded yet");

        demo.record(1);
        demo.record(2);
        assert_eq!(
            demo.start_job(),
            Some(ComputeJob::Demo { batch: vec![1, 2] })
        );
        assert!(demo.start_job().is_none(), "slot is busy");

        demo.record(3);
        assert!(demo.start_job().is_none(), "still busy with new work");
    }

    #[test]
    fn reruns_after_work_arrives_mid_job() {
        let mut demo = Demo::new(1000);
        demo.record(1);
        let _job = demo.start_job().unwrap();
        demo.record(2); // arrives while the job runs

        demo.accept(Reduction { count: 1, sum: 1 });
        assert_eq!(demo.latest(), Some(Reduction { count: 1, sum: 1 }));
        // The slot is idle and work is pending, so a rerun launches with
        // the full batch.
        assert_eq!(
            demo.start_job(),
            Some(ComputeJob::Demo { batch: vec![1, 2] })
        );
    }

    #[test]
    fn failure_frees_the_slot_and_keeps_the_last_result() {
        let mut demo = Demo::new(1000);
        demo.record(1);
        let _job = demo.start_job().unwrap();
        demo.accept(Reduction { count: 1, sum: 1 });

        demo.record(2);
        let _rerun = demo.start_job().unwrap();
        demo.record_failure();

        // The last good reduction is retained, and the slot is free again.
        assert_eq!(demo.latest(), Some(Reduction { count: 1, sum: 1 }));
        assert!(!demo.is_running());
        // The failed batch is not retried on its own (no tight loop on a
        // deterministic failure); the next sample triggers a fresh job that
        // re-reduces the full accumulation, recovering the lost work.
        assert!(!demo.can_start(), "no new work pending yet");
        demo.record(3);
        assert_eq!(
            demo.start_job(),
            Some(ComputeJob::Demo {
                batch: vec![1, 2, 3]
            })
        );
    }
}
