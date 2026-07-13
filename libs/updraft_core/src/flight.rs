//! Flight state for own position and the flown trace.
//!
//! Trace statistics are computed asynchronously and invalidated when the
//! trace is cleared.

use std::time::Duration;

use updraft_geo::LatLon;
use updraft_units::{Angle, Length, Speed};

use crate::job::ComputeSlot;
use crate::protocol::{
    Change as AppChange, ComputeCancellation, ComputeJob as AppComputeJob,
    ComputeKind as AppComputeKind, Effect, Update,
};
use crate::time::{Timer, Timers};

/// An altitude above mean sea level.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct MslAltitude(Length);

impl MslAltitude {
    pub const ZERO: Self = Self(Length::ZERO);

    pub const fn new(length: Length) -> Self {
        Self(length)
    }

    pub const fn length(self) -> Length {
        self.0
    }
}

/// Tuning knobs for the flight domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Minimum spacing between two trace-statistics compute jobs.
    pub trace_stats_interval: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trace_stats_interval: Duration::from_secs(5),
        }
    }
}

/// A normalized own-position observation from a positioning source.
///
/// This doubles as the published kinematic state vector: clients use it
/// to estimate the current render position, so frame-rate animation never
/// crosses the transport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionFix {
    /// Clock time when the fix was observed.
    pub observed_at: Duration,
    pub position: LatLon,
    pub altitude: Option<MslAltitude>,
    /// Track over ground.
    pub track: Option<Angle>,
    pub ground_speed: Option<Speed>,
}

/// Statistics over the flown trace, computed by a runtime worker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceStats {
    pub fix_count: u64,
    /// Ground distance flown along the trace.
    pub distance: Length,
    pub max_altitude: Option<MslAltitude>,
}

/// A recorded event or request owned by the flight domain.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    /// A user command.
    Command(Command),
    /// A normalized sensor observation.
    Observation(Observation),
}

impl Input {
    pub(crate) fn observed_at(&self) -> Option<Duration> {
        match self {
            Self::Command(_) => None,
            Self::Observation(observation) => Some(observation.observed_at()),
        }
    }
}

/// A recorded user command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Clears the flown trace and its statistics.
    ClearTrace,
}

/// A normalized sensor observation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Observation {
    /// An own-position fix.
    Position(PositionFix),
}

impl Observation {
    fn observed_at(&self) -> Duration {
        match self {
            Self::Position(fix) => fix.observed_at,
        }
    }
}

/// Requests the current own-position fix.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GetPosition;

impl crate::Query for GetPosition {
    type Output = Option<PositionFix>;

    fn execute(self, app: &crate::App) -> Self::Output {
        app.flight.position()
    }
}

/// Requests the most recent trace statistics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GetTraceStats;

impl crate::Query for GetTraceStats {
    type Output = Option<TraceStats>;

    fn execute(self, app: &crate::App) -> Self::Output {
        app.flight.trace_stats()
    }
}

/// A client-visible flight-state update.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// The own-position last-value update.
    Position(PositionFix),
    /// New trace statistics, or `None` after the trace was cleared.
    TraceStats(Option<TraceStats>),
}

/// One expensive flight calculation, carrying a snapshot of everything it
/// needs.
#[derive(Clone, Debug, PartialEq)]
pub enum ComputeJob {
    /// Statistics over the flown trace.
    TraceStats {
        revision: crate::ComputeRevision,
        fixes: Vec<PositionFix>,
    },
}

impl ComputeJob {
    pub fn kind(&self) -> ComputeKind {
        match self {
            Self::TraceStats { .. } => ComputeKind::TraceStats,
        }
    }

    pub fn revision(&self) -> crate::ComputeRevision {
        match self {
            Self::TraceStats { revision, .. } => *revision,
        }
    }

    pub fn run(self) -> ComputeResult {
        match self {
            Self::TraceStats { revision, fixes } => ComputeResult::TraceStats {
                revision,
                stats: trace_stats(&fixes),
            },
        }
    }
}

/// The kind of a flight compute job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComputeKind {
    TraceStats,
}

/// A completed flight compute job.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ComputeResult {
    TraceStats {
        revision: crate::ComputeRevision,
        stats: TraceStats,
    },
}

impl ComputeResult {
    pub fn kind(&self) -> ComputeKind {
        match self {
            Self::TraceStats { .. } => ComputeKind::TraceStats,
        }
    }

    pub fn revision(&self) -> crate::ComputeRevision {
        match self {
            Self::TraceStats { revision, .. } => *revision,
        }
    }
}

/// The shared current flight state for a newly subscribing client.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub position: Option<PositionFix>,
    pub trace_stats: Option<TraceStats>,
}

/// Computes trace statistics using a WGS84 geodesic solve for each segment.
///
/// Its cost grows with the trace, so it runs on a compute worker.
pub(crate) fn trace_stats(fixes: &[PositionFix]) -> TraceStats {
    let distance = fixes
        .windows(2)
        .map(|pair| pair[0].position.distance(pair[1].position))
        .fold(Length::ZERO, |total, leg| total + leg);
    let max_altitude = fixes
        .iter()
        .filter_map(|fix| fix.altitude)
        .reduce(|a, b| if b > a { b } else { a });
    TraceStats {
        fix_count: fixes.len() as u64,
        distance,
        max_altitude,
    }
}

/// The flight domain state.
#[derive(Debug)]
pub(crate) struct Flight {
    /// Minimum spacing between two trace-statistics job starts.
    stats_interval: Duration,
    position: Option<PositionFix>,
    trace: Vec<PositionFix>,
    trace_stats: Option<TraceStats>,
    stats_job: ComputeSlot,
    stats_started_at: Option<Duration>,
}

impl Flight {
    pub(crate) fn new(config: Config) -> Self {
        Self {
            stats_interval: config.trace_stats_interval,
            position: None,
            trace: Vec::new(),
            trace_stats: None,
            stats_job: ComputeSlot::default(),
            stats_started_at: None,
        }
    }

    pub(crate) fn position(&self) -> Option<PositionFix> {
        self.position
    }

    pub(crate) fn trace_stats(&self) -> Option<TraceStats> {
        self.trace_stats
    }

    pub(crate) fn handle(
        &mut self,
        input: Input,
        clock_time: Duration,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        match input {
            Input::Command(Command::ClearTrace) => self.clear_trace(timers, update),
            Input::Observation(Observation::Position(fix)) => {
                self.observe_position(fix, clock_time, timers, update);
            }
        }
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot {
            position: self.position(),
            trace_stats: self.trace_stats(),
        }
    }

    pub(crate) fn observe_position(
        &mut self,
        fix: PositionFix,
        clock_time: Duration,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        self.position = Some(fix);
        self.trace.push(fix);
        update
            .changes
            .push(AppChange::Flight(Change::Position(fix)));
        self.stats_job.request();
        self.schedule_stats(clock_time, timers);
    }

    pub(crate) fn clear_trace(&mut self, timers: &mut Timers, update: &mut Update) {
        self.trace.clear();
        if let Some(revision) = self.stats_job.invalidate() {
            update
                .effects
                .push(Effect::CancelCompute(ComputeCancellation {
                    kind: AppComputeKind::Flight(ComputeKind::TraceStats),
                    revision,
                }));
        }
        timers.cancel(Timer::TraceStats);
        if self.trace_stats.take().is_some() {
            update
                .changes
                .push(AppChange::Flight(Change::TraceStats(None)));
        }
    }

    pub(crate) fn timer(&mut self, timer: Timer, clock_time: Duration, update: &mut Update) {
        match timer {
            Timer::TraceStats => self.start_stats(clock_time, update),
        }
    }

    pub(crate) fn compute_result(
        &mut self,
        result: ComputeResult,
        clock_time: Duration,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        match result {
            ComputeResult::TraceStats { revision, stats } => {
                if self.stats_job.finish(revision) {
                    self.trace_stats = Some(stats);
                    update
                        .changes
                        .push(AppChange::Flight(Change::TraceStats(Some(stats))));
                }
                self.schedule_stats(clock_time, timers);
            }
        }
    }

    pub(crate) fn compute_failed(
        &mut self,
        kind: ComputeKind,
        revision: crate::ComputeRevision,
        clock_time: Duration,
        timers: &mut Timers,
    ) {
        self.finish_compute(kind, revision, clock_time, timers);
    }

    pub(crate) fn compute_cancelled(
        &mut self,
        kind: ComputeKind,
        revision: crate::ComputeRevision,
        clock_time: Duration,
        timers: &mut Timers,
    ) {
        self.finish_compute(kind, revision, clock_time, timers);
    }

    fn finish_compute(
        &mut self,
        kind: ComputeKind,
        revision: crate::ComputeRevision,
        clock_time: Duration,
        timers: &mut Timers,
    ) {
        match kind {
            ComputeKind::TraceStats => {
                // An older trace-statistics result stays safe to show, so
                // a non-result only frees the slot. New fixes trigger the
                // next attempt.
                self.stats_job.finish(revision);
                self.schedule_stats(clock_time, timers);
            }
        }
    }

    /// Starts the requested trace-statistics job, carrying a snapshot of
    /// the trace and the current compute revision.
    fn start_stats(&mut self, clock_time: Duration, update: &mut Update) {
        if !self.stats_job.wants_start() {
            return;
        }
        let revision = self.stats_job.start();
        self.stats_started_at = Some(clock_time);
        update.effects.push(Effect::Compute(AppComputeJob::Flight(
            ComputeJob::TraceStats {
                revision,
                fixes: self.trace.clone(),
            },
        )));
    }

    /// Schedules the timer that starts the next job, at least
    /// [`stats_interval`](Self::stats_interval) after the previous start.
    fn schedule_stats(&mut self, clock_time: Duration, timers: &mut Timers) {
        if !self.stats_job.wants_start() || timers.is_scheduled(Timer::TraceStats) {
            return;
        }
        let at = match self.stats_started_at {
            Some(started) => started.saturating_add(self.stats_interval).max(clock_time),
            None => clock_time,
        };
        timers.schedule(Timer::TraceStats, at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(latitude: f64, longitude: f64, altitude: Option<f64>) -> PositionFix {
        PositionFix {
            observed_at: Duration::ZERO,
            position: LatLon::from_degrees(latitude, longitude),
            altitude: altitude.map(|meters| MslAltitude::new(Length::from_meters(meters))),
            track: None,
            ground_speed: None,
        }
    }

    #[test]
    fn trace_stats_over_empty_trace() {
        let stats = trace_stats(&[]);
        assert_eq!(stats.fix_count, 0);
        assert_eq!(stats.distance, Length::ZERO);
        assert_eq!(stats.max_altitude, None);
    }

    #[test]
    fn trace_stats_sums_geodesic_legs() {
        // Two one-degree meridian arcs, roughly 110.6 km each.
        let fixes = [
            fix(50., 6., Some(1000.)),
            fix(51., 6., None),
            fix(52., 6., Some(1500.)),
        ];
        let stats = trace_stats(&fixes);
        assert_eq!(stats.fix_count, 3);
        assert!((stats.distance.as_kilometers() - 222.6).abs() < 1.);
        assert_eq!(
            stats.max_altitude,
            Some(MslAltitude::new(Length::from_meters(1500.)))
        );
    }

    #[test]
    fn input_reports_observation_time() {
        let position = fix(50., 6., Some(1000.));

        assert_eq!(
            Input::Observation(Observation::Position(position)).observed_at(),
            Some(position.observed_at)
        );
        assert_eq!(Input::Command(Command::ClearTrace).observed_at(), None);
    }
}
