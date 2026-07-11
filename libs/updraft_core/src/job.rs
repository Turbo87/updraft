use serde::{Deserialize, Serialize};

/// A generation counter for compute-worker invalidation.
///
/// Some state changes make all earlier background work invalid, such as
/// clearing the trace or seeking to a distant replay position. These
/// changes increase the epoch. The core ignores a result from an older
/// epoch, and the runtime clears a worker's cached state when it sees a
/// new epoch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Epoch(u64);

impl Epoch {
    fn bump(&mut self) {
        self.0 += 1;
    }
}

/// Tracks one compute-worker kind from the core side.
///
/// Each worker kind runs at most one job at a time. The slot records
/// whether a job is running, whether another run is needed, and which
/// epoch results are still valid for. Domains provide job inputs, rules
/// for rejecting old results, and code that applies valid results. The
/// lifecycle stays inside this slot and the runtime worker adapter.
#[derive(Debug, Default)]
pub(crate) struct JobSlot {
    epoch: Epoch,
    running: bool,
    pending: bool,
}

impl JobSlot {
    /// Records that state changed and a (re)computation is needed.
    pub(crate) fn request(&mut self) {
        self.pending = true;
    }

    /// Whether a new job should be started now.
    pub(crate) fn wants_start(&self) -> bool {
        self.pending && !self.running
    }

    /// Marks the requested job as running and returns the epoch to tag
    /// it with. Only call when [`wants_start`](Self::wants_start) is true.
    pub(crate) fn start(&mut self) -> Epoch {
        debug_assert!(self.wants_start());
        self.pending = false;
        self.running = true;
        self.epoch
    }

    /// Records that the in-flight job completed or failed. Returns whether
    /// the result belongs to the current epoch and may be applied.
    pub(crate) fn finish(&mut self, epoch: Epoch) -> bool {
        self.running = false;
        epoch == self.epoch
    }

    /// Invalidates all earlier work: an in-flight job's result will be
    /// rejected, and any pending request is discarded.
    pub(crate) fn invalidate(&mut self) {
        self.epoch.bump();
        self.pending = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_lifecycle() {
        let mut slot = JobSlot::default();
        assert!(!slot.wants_start());

        slot.request();
        assert!(slot.wants_start());
        let epoch = slot.start();
        assert!(!slot.wants_start());

        // More work arrives while the job is running.
        slot.request();
        assert!(!slot.wants_start());

        assert!(slot.finish(epoch));
        assert!(slot.wants_start());
    }

    #[test]
    fn invalidation_discards_a_pending_request() {
        let mut slot = JobSlot::default();
        slot.request();
        assert!(slot.wants_start());

        // Invalidating before the job ever starts drops the pending request
        // so nothing runs over the now-stale state.
        slot.invalidate();
        assert!(!slot.wants_start());

        // A fresh request still runs, under the bumped epoch.
        slot.request();
        let epoch = slot.start();
        assert!(slot.finish(epoch));
    }

    #[test]
    fn invalidation_rejects_in_flight_results() {
        let mut slot = JobSlot::default();
        slot.request();
        let epoch = slot.start();

        slot.invalidate();
        assert!(!slot.finish(epoch));
        assert!(!slot.wants_start());

        slot.request();
        let next = slot.start();
        assert_ne!(epoch, next);
        assert!(slot.finish(next));
    }
}
