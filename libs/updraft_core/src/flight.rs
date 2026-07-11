//! The flight domain: own position and the flown trace.
//!
//! A deliberately small domain module. It owns the current own-position
//! fix (a last-value change) and the flown trace,
//! whose statistics are the first user of the asynchronous compute path:
//! every new fix requests a recomputation, the core throttles job starts
//! via a timer to one per configured interval
//! ([`AppConfig::trace_stats_interval`](crate::AppConfig)), and clearing
//! the trace bumps the epoch so an in-flight result is rejected instead
//! of applied.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use updraft_geo::LatLon;
use updraft_units::{Angle, Length, Speed};

use crate::job::JobSlot;
use crate::protocol::{
    Change, ComputeFailure, ComputeJob, ComputeKind, ComputeResult, Effect, Update,
};
use crate::time::{MonotonicTime, Timer, Timers};

/// A normalized own-position observation from a positioning source.
///
/// This doubles as the published kinematic state vector: clients use it
/// to estimate the current render position, so frame-rate animation never
/// crosses the transport.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PositionFix {
    /// When the fix was observed, stamped by the adapter from the
    /// process-wide monotonic timeline.
    pub observed_at: MonotonicTime,
    pub position: LatLon,
    /// Altitude above mean sea level.
    pub altitude: Option<Length>,
    /// Track over ground.
    pub track: Option<Angle>,
    pub ground_speed: Option<Speed>,
}

/// Statistics over the flown trace, computed by a runtime worker.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceStats {
    pub fix_count: u64,
    /// Ground distance flown along the trace.
    pub distance: Length,
    pub max_altitude: Option<Length>,
}

/// Computes [`TraceStats`] over a trace snapshot.
///
/// Pure and deliberately brute-force: a full WGS84 geodesic solve per
/// segment over the whole trace. It stands in for the genuinely expensive
/// calculations (live scoring, glide reach) that will use the same worker
/// path, and grows linearly with the flight so it must not run on the
/// input loop.
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
    stats_job: JobSlot,
    stats_started_at: Option<MonotonicTime>,
}

impl Flight {
    pub(crate) fn new(stats_interval: Duration) -> Self {
        Self {
            stats_interval,
            position: None,
            trace: Vec::new(),
            trace_stats: None,
            stats_job: JobSlot::default(),
            stats_started_at: None,
        }
    }

    pub(crate) fn position(&self) -> Option<PositionFix> {
        self.position
    }

    pub(crate) fn trace_stats(&self) -> Option<TraceStats> {
        self.trace_stats
    }

    pub(crate) fn observe_position(
        &mut self,
        fix: PositionFix,
        now: MonotonicTime,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        self.position = Some(fix);
        self.trace.push(fix);
        update.changes.push(Change::Position(fix));
        self.stats_job.request();
        self.schedule_stats(now, timers);
    }

    pub(crate) fn clear_trace(&mut self, timers: &mut Timers, update: &mut Update) {
        self.trace.clear();
        self.stats_job.invalidate();
        timers.cancel(Timer::TraceStats);
        if self.trace_stats.take().is_some() {
            update.changes.push(Change::TraceStats(None));
        }
    }

    pub(crate) fn timer(&mut self, timer: Timer, now: MonotonicTime, update: &mut Update) {
        match timer {
            Timer::TraceStats => self.start_stats(now, update),
        }
    }

    pub(crate) fn compute_result(
        &mut self,
        result: ComputeResult,
        now: MonotonicTime,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        match result {
            ComputeResult::TraceStats { epoch, stats } => {
                if self.stats_job.finish(epoch) {
                    self.trace_stats = Some(stats);
                    update.changes.push(Change::TraceStats(Some(stats)));
                }
                self.schedule_stats(now, timers);
            }
        }
    }

    pub(crate) fn compute_failed(
        &mut self,
        failure: &ComputeFailure,
        now: MonotonicTime,
        timers: &mut Timers,
    ) {
        match failure.kind {
            ComputeKind::TraceStats => {
                // An older trace-statistics result stays safe to show, so
                // the failure only frees the slot. New fixes trigger the
                // next attempt.
                self.stats_job.finish(failure.epoch);
                self.schedule_stats(now, timers);
            }
        }
    }

    /// Starts the requested trace-statistics job, carrying a snapshot of
    /// the trace and the current epoch.
    fn start_stats(&mut self, now: MonotonicTime, update: &mut Update) {
        if !self.stats_job.wants_start() {
            return;
        }
        let epoch = self.stats_job.start();
        self.stats_started_at = Some(now);
        update.effects.push(Effect::Compute(ComputeJob::TraceStats {
            epoch,
            fixes: self.trace.clone(),
        }));
    }

    /// Schedules the timer that starts the next job, at least
    /// [`stats_interval`](Self::stats_interval) after the previous start.
    fn schedule_stats(&mut self, now: MonotonicTime, timers: &mut Timers) {
        if !self.stats_job.wants_start() || timers.is_scheduled(Timer::TraceStats) {
            return;
        }
        let at = match self.stats_started_at {
            Some(started) => (started + self.stats_interval).max(now),
            None => now,
        };
        timers.schedule(Timer::TraceStats, at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(latitude: f64, longitude: f64, altitude: Option<f64>) -> PositionFix {
        PositionFix {
            observed_at: MonotonicTime::ORIGIN,
            position: LatLon::from_degrees(latitude, longitude),
            altitude: altitude.map(Length::from_meters),
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
        assert_eq!(stats.max_altitude, Some(Length::from_meters(1500.)));
    }
}
