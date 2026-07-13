/// A revision counter for compute-worker invalidation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ComputeRevision(u64);

impl ComputeRevision {
    fn bump(&mut self) {
        self.0 += 1;
    }
}

/// Tracks one compute-job kind from the core side.
#[derive(Debug, Default)]
pub(crate) struct ComputeSlot {
    revision: ComputeRevision,
    running: bool,
    pending: bool,
}

impl ComputeSlot {
    pub(crate) fn request(&mut self) {
        self.pending = true;
    }

    pub(crate) fn wants_start(&self) -> bool {
        self.pending && !self.running
    }

    pub(crate) fn start(&mut self) -> ComputeRevision {
        debug_assert!(self.wants_start());
        self.pending = false;
        self.running = true;
        self.revision
    }

    pub(crate) fn finish(&mut self, revision: ComputeRevision) -> bool {
        self.running = false;
        revision == self.revision
    }

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

        slot.request();
        assert!(!slot.wants_start());

        assert!(slot.finish(revision));
        assert!(slot.wants_start());
    }

    #[test]
    fn invalidation_rejects_in_flight_results() {
        let mut slot = ComputeSlot::default();
        slot.request();
        let revision = slot.start();

        slot.invalidate();
        assert!(!slot.finish(revision));

        slot.request();
        let next = slot.start();
        assert_ne!(revision, next);
        assert!(slot.finish(next));
    }
}
