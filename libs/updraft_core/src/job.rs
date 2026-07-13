/// Identifies the state used to create a compute job.
///
/// Results from older revisions are ignored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ComputeRevision(u64);

impl ComputeRevision {
    fn bump(&mut self) {
        self.0 += 1;
    }
}

/// Tracks pending and running compute work.
#[derive(Debug, Default)]
pub(crate) struct ComputeSlot {
    revision: ComputeRevision,
    running: bool,
    pending: bool,
}

impl ComputeSlot {
    /// Records that state changed and a (re)computation is needed.
    pub(crate) fn request(&mut self) {
        self.pending = true;
    }

    /// Whether a new job should be started now.
    pub(crate) fn wants_start(&self) -> bool {
        self.pending && !self.running
    }

    /// Marks the requested job as running and returns the revision to tag
    /// it with. Only call when [`wants_start()`](Self::wants_start) is true.
    pub(crate) fn start(&mut self) -> ComputeRevision {
        debug_assert!(self.wants_start());
        self.pending = false;
        self.running = true;
        self.revision
    }

    /// Records that the in-flight job completed or failed. Returns whether
    /// the result belongs to the current revision and may be applied.
    pub(crate) fn finish(&mut self, revision: ComputeRevision) -> bool {
        self.running = false;
        revision == self.revision
    }

    /// Invalidates all earlier work: an in-flight job's result will be
    /// rejected, and any pending request is discarded.
    pub(crate) fn invalidate(&mut self) {
        self.revision.bump();
        self.pending = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_lifecycle() {
        let mut slot = ComputeSlot::default();
        assert!(!slot.wants_start());

        slot.request();
        assert!(slot.wants_start());
        let revision = slot.start();
        assert!(!slot.wants_start());

        // More work arrives while the job is running.
        slot.request();
        assert!(!slot.wants_start());

        assert!(slot.finish(revision));
        assert!(slot.wants_start());
    }

    #[test]
    fn invalidation_discards_a_pending_request() {
        let mut slot = ComputeSlot::default();
        slot.request();
        assert!(slot.wants_start());

        // Invalidating before the job ever starts drops the pending request
        // so nothing runs over the now-stale state.
        slot.invalidate();
        assert!(!slot.wants_start());

        // A fresh request still runs under the new revision.
        slot.request();
        let revision = slot.start();
        assert!(slot.finish(revision));
    }

    #[test]
    fn invalidation_rejects_in_flight_results() {
        let mut slot = ComputeSlot::default();
        slot.request();
        let revision = slot.start();

        slot.invalidate();
        assert!(!slot.finish(revision));
        assert!(!slot.wants_start());

        slot.request();
        let next = slot.start();
        assert_ne!(revision, next);
        assert!(slot.finish(next));
    }
}
