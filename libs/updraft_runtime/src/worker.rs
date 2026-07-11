use updraft_core::{ComputeJob, ComputeResult};

/// Executes compute jobs for one [`ComputeKind`](updraft_core::ComputeKind)
/// on a dedicated runtime thread.
///
/// The core's job slot guarantees at most one job at a time per kind. A
/// worker may keep cached intermediate data between runs (a live optimizer
/// can extend its work over the growing flight trace instead of starting
/// from nothing). That data is a cache, not authoritative state. The
/// runtime calls [`reset`](Self::reset) when the job epoch changes and
/// after a failure, so a job never observes cache from an invalidated
/// generation or a poisoned run.
pub trait Worker: Send + 'static {
    /// Runs one job to completion.
    ///
    /// An `Err` (or a panic) becomes a typed
    /// [`Input::ComputeFailed`](updraft_core::Input) for the core, so the
    /// job slot never waits forever.
    fn run(&mut self, job: ComputeJob) -> Result<ComputeResult, String>;

    /// Drops all cached state.
    ///
    /// The runtime also calls this to recover after [`run`](Self::run) or a
    /// previous `reset` panicked, so it must restore a valid empty state
    /// even when a panic left `self` torn partway through a mutation.
    fn reset(&mut self) {}
}

/// A stateless worker that runs the job's own pure calculation
/// ([`ComputeJob::run`]).
#[derive(Debug, Default)]
pub struct PureWorker;

impl Worker for PureWorker {
    fn run(&mut self, job: ComputeJob) -> Result<ComputeResult, String> {
        Ok(job.run())
    }
}
