//! Flight state for own position and the flown trace.
//!
//! Trace statistics are computed asynchronously and invalidated when the
//! trace is cleared.

use crate::device::DeviceId;
use crate::job::ComputeSlot;
use crate::protocol::{
    Change as AppChange, ComputeCancellation, ComputeJob as AppComputeJob,
    ComputeKind as AppComputeKind, Effect, Update,
};
use crate::time::{Timer, Timers};
use std::collections::HashMap;
use std::time::Duration;
use updraft_geo::LatLon;
use updraft_units::{Angle, Length, MslAltitude, PressureAltitude, Speed};

/// Tuning knobs for the flight domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlightConfig {
    /// Minimum spacing between two trace-statistics compute jobs.
    pub trace_stats_interval: Duration,
}

impl Default for FlightConfig {
    fn default() -> Self {
        Self {
            trace_stats_interval: Duration::from_secs(5),
        }
    }
}

/// The stable identity of a normalized flight-data source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceId {
    /// Built-in platform GNSS and pressure sensors.
    Internal,
    /// One configured external device.
    External(DeviceId),
    /// Interactive simulator mode.
    Simulator,
    /// Replay of a recorded flight.
    Replay,
}

/// A value attributed to one flight-data source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sourced<T> {
    pub source: SourceId,
    pub value: T,
}

impl<T> Sourced<T> {
    pub const fn new(source: SourceId, value: T) -> Self {
        Self { source, value }
    }

    /// Creates a value sourced from the built-in platform sensors.
    pub const fn internal(value: T) -> Self {
        Self::new(SourceId::Internal, value)
    }

    /// Creates a value sourced from one configured external device.
    pub const fn external(device_id: DeviceId, value: T) -> Self {
        Self::new(SourceId::External(device_id), value)
    }

    /// Creates a value sourced from interactive simulator mode.
    pub const fn simulator(value: T) -> Self {
        Self::new(SourceId::Simulator, value)
    }

    /// Creates a value sourced from a recorded-flight replay.
    pub const fn replay(value: T) -> Self {
        Self::new(SourceId::Replay, value)
    }
}

/// A value captured at a monotonic observation time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation<T> {
    pub observed_at: Duration,
    pub value: T,
}

impl<T> Observation<T> {
    pub const fn new(observed_at: Duration, value: T) -> Self {
        Self { observed_at, value }
    }
}

/// Components reported together in one GNSS position update.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GnssUpdate {
    pub position: LatLon,
    /// Mean-sea-level GNSS altitude.
    pub altitude: Option<MslAltitude>,
    /// Track over ground.
    pub track: Option<Angle>,
    pub ground_speed: Option<Speed>,
}

/// GNSS components retained with their individual observation times.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GnssState {
    pub position: Observation<LatLon>,
    /// Mean-sea-level GNSS altitude.
    pub altitude: Option<Observation<MslAltitude>>,
    /// Track over ground.
    pub track: Option<Observation<Angle>>,
    pub ground_speed: Option<Observation<Speed>>,
}

impl From<Observation<GnssUpdate>> for GnssState {
    fn from(observation: Observation<GnssUpdate>) -> Self {
        let observed_at = observation.observed_at;
        let update = observation.value;
        Self {
            position: Observation::new(observed_at, update.position),
            altitude: update
                .altitude
                .map(|altitude| Observation::new(observed_at, altitude)),
            track: update
                .track
                .map(|track| Observation::new(observed_at, track)),
            ground_speed: update
                .ground_speed
                .map(|speed| Observation::new(observed_at, speed)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SourceState {
    gnss: Option<GnssState>,
    pressure_altitude: Option<Observation<PressureAltitude>>,
}

impl SourceState {
    fn observe_gnss(&mut self, observation: Observation<GnssUpdate>) -> Option<GnssState> {
        let gnss = GnssState::from(observation);
        if self
            .gnss
            .is_some_and(|current| gnss.position.observed_at < current.position.observed_at)
        {
            return None;
        }
        let gnss = self.gnss.map_or(gnss, |current| GnssState {
            altitude: gnss.altitude.or(current.altitude),
            track: gnss.track.or(current.track),
            ground_speed: gnss.ground_speed.or(current.ground_speed),
            ..gnss
        });
        self.gnss = Some(gnss);
        Some(gnss)
    }

    fn observe_pressure_altitude(&mut self, observation: Observation<PressureAltitude>) -> bool {
        if self
            .pressure_altitude
            .is_some_and(|current| observation.observed_at < current.observed_at)
        {
            return false;
        }
        self.pressure_altitude = Some(observation);
        true
    }
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
pub enum FlightInput {
    /// Clears the flown trace and its statistics.
    ClearTrace,
    /// A GNSS position update.
    Gnss(Sourced<Observation<GnssUpdate>>),
    /// A standard-pressure altitude observation.
    PressureAltitude(Sourced<Observation<PressureAltitude>>),
}

impl FlightInput {
    pub(crate) fn observed_at(&self) -> Option<Duration> {
        match self {
            Self::ClearTrace => None,
            Self::Gnss(gnss) => Some(gnss.value.observed_at),
            Self::PressureAltitude(altitude) => Some(altitude.value.observed_at),
        }
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
pub enum FlightChange {
    /// The selected GNSS component state.
    Gnss(GnssState),
    /// The standard-pressure altitude last-value update.
    PressureAltitude(PressureAltitude),
    /// New trace statistics, or `None` after the trace was cleared.
    TraceStats(Option<TraceStats>),
}

/// One expensive flight calculation, carrying a snapshot of everything it
/// needs.
#[derive(Clone, Debug, PartialEq)]
pub enum FlightComputeJob {
    /// Statistics over the flown trace.
    TraceStats {
        revision: crate::ComputeRevision,
        fixes: Vec<GnssState>,
    },
}

impl FlightComputeJob {
    pub fn kind(&self) -> FlightComputeKind {
        match self {
            Self::TraceStats { .. } => FlightComputeKind::TraceStats,
        }
    }

    pub fn revision(&self) -> crate::ComputeRevision {
        match self {
            Self::TraceStats { revision, .. } => *revision,
        }
    }

    pub fn run(self) -> FlightComputeResult {
        match self {
            Self::TraceStats { revision, fixes } => FlightComputeResult::TraceStats {
                revision,
                stats: trace_stats(&fixes),
            },
        }
    }
}

/// The kind of a flight compute job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlightComputeKind {
    TraceStats,
}

/// A completed flight compute job.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlightComputeResult {
    TraceStats {
        revision: crate::ComputeRevision,
        stats: TraceStats,
    },
}

impl FlightComputeResult {
    pub fn kind(&self) -> FlightComputeKind {
        match self {
            Self::TraceStats { .. } => FlightComputeKind::TraceStats,
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
pub struct FlightSnapshot {
    pub gnss: Option<GnssState>,
    pub pressure_altitude: Option<PressureAltitude>,
    pub trace_stats: Option<TraceStats>,
}

/// Computes trace statistics using a WGS84 geodesic solve for each segment.
///
/// Its cost grows with the trace, so it runs on a compute worker.
pub(crate) fn trace_stats(fixes: &[GnssState]) -> TraceStats {
    let distance = fixes
        .windows(2)
        .map(|pair| pair[0].position.value.distance(pair[1].position.value))
        .fold(Length::ZERO, |total, leg| total + leg);
    let max_altitude = fixes
        .iter()
        .filter_map(|fix| fix.altitude.map(|altitude| altitude.value))
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
    source_states: HashMap<SourceId, SourceState>,
    selected_gnss_source: Option<SourceId>,
    selected_pressure_altitude_source: Option<SourceId>,
    trace: Vec<GnssState>,
    trace_stats: Option<TraceStats>,
    stats_job: ComputeSlot,
    stats_started_at: Option<Duration>,
}

impl Flight {
    pub(crate) fn new(config: FlightConfig) -> Self {
        Self {
            stats_interval: config.trace_stats_interval,
            source_states: HashMap::new(),
            selected_gnss_source: None,
            selected_pressure_altitude_source: None,
            trace: Vec::new(),
            trace_stats: None,
            stats_job: ComputeSlot::default(),
            stats_started_at: None,
        }
    }

    pub(crate) fn trace_stats(&self) -> Option<TraceStats> {
        self.trace_stats
    }

    pub(crate) fn handle(
        &mut self,
        input: FlightInput,
        clock_time: Duration,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        match input {
            FlightInput::ClearTrace => self.clear_trace(timers, update),
            FlightInput::Gnss(gnss) => self.observe_gnss(gnss, clock_time, timers, update),
            FlightInput::PressureAltitude(altitude) => {
                self.observe_pressure_altitude(altitude, update)
            }
        }
    }

    pub(crate) fn snapshot(&self) -> FlightSnapshot {
        FlightSnapshot {
            gnss: self.selected_gnss(),
            pressure_altitude: self
                .selected_pressure_altitude()
                .map(|observation| observation.value),
            trace_stats: self.trace_stats(),
        }
    }

    fn selected_gnss(&self) -> Option<GnssState> {
        self.selected_gnss_source
            .and_then(|source| self.source_states.get(&source))
            .and_then(|state| state.gnss)
    }

    fn selected_pressure_altitude(&self) -> Option<Observation<PressureAltitude>> {
        self.selected_pressure_altitude_source
            .and_then(|source| self.source_states.get(&source))
            .and_then(|state| state.pressure_altitude)
    }

    pub(crate) fn observe_gnss(
        &mut self,
        observation: Sourced<Observation<GnssUpdate>>,
        clock_time: Duration,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        let source = observation.source;
        let observation = observation.value;
        let selected_observed_at = self.selected_gnss().map(|gnss| gnss.position.observed_at);
        let Some(gnss) = self
            .source_states
            .entry(source)
            .or_default()
            .observe_gnss(observation)
        else {
            return;
        };
        if selected_observed_at.is_none_or(|selected| observation.observed_at >= selected) {
            self.selected_gnss_source = Some(source);
        }
        if self.selected_gnss_source != Some(source) {
            return;
        }

        self.trace.push(gnss);
        update
            .changes
            .push(AppChange::Flight(FlightChange::Gnss(gnss)));
        self.stats_job.request();
        self.schedule_stats(clock_time, timers);
    }

    fn observe_pressure_altitude(
        &mut self,
        observation: Sourced<Observation<PressureAltitude>>,
        update: &mut Update,
    ) {
        let source = observation.source;
        let observation = observation.value;
        let selected_observed_at = self
            .selected_pressure_altitude()
            .map(|altitude| altitude.observed_at);
        if !self
            .source_states
            .entry(source)
            .or_default()
            .observe_pressure_altitude(observation)
        {
            return;
        }
        if selected_observed_at.is_none_or(|selected| observation.observed_at >= selected) {
            self.selected_pressure_altitude_source = Some(source);
        }
        if self.selected_pressure_altitude_source != Some(source) {
            return;
        }

        let change = FlightChange::PressureAltitude(observation.value);
        update.changes.push(AppChange::Flight(change));
    }

    pub(crate) fn clear_trace(&mut self, timers: &mut Timers, update: &mut Update) {
        self.trace.clear();
        if let Some(revision) = self.stats_job.invalidate() {
            update
                .effects
                .push(Effect::CancelCompute(ComputeCancellation {
                    kind: AppComputeKind::Flight(FlightComputeKind::TraceStats),
                    revision,
                }));
        }
        timers.cancel(Timer::TraceStats);
        if self.trace_stats.take().is_some() {
            update
                .changes
                .push(AppChange::Flight(FlightChange::TraceStats(None)));
        }
    }

    pub(crate) fn timer(&mut self, timer: Timer, clock_time: Duration, update: &mut Update) {
        match timer {
            Timer::TraceStats => self.start_stats(clock_time, update),
        }
    }

    pub(crate) fn compute_result(
        &mut self,
        result: FlightComputeResult,
        clock_time: Duration,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        match result {
            FlightComputeResult::TraceStats { revision, stats } => {
                if self.stats_job.finish(revision) {
                    self.trace_stats = Some(stats);
                    update
                        .changes
                        .push(AppChange::Flight(FlightChange::TraceStats(Some(stats))));
                }
                self.schedule_stats(clock_time, timers);
            }
        }
    }

    pub(crate) fn compute_failed(
        &mut self,
        kind: FlightComputeKind,
        revision: crate::ComputeRevision,
        clock_time: Duration,
        timers: &mut Timers,
    ) {
        self.finish_compute(kind, revision, clock_time, timers);
    }

    pub(crate) fn compute_cancelled(
        &mut self,
        kind: FlightComputeKind,
        revision: crate::ComputeRevision,
        clock_time: Duration,
        timers: &mut Timers,
    ) {
        self.finish_compute(kind, revision, clock_time, timers);
    }

    fn finish_compute(
        &mut self,
        kind: FlightComputeKind,
        revision: crate::ComputeRevision,
        clock_time: Duration,
        timers: &mut Timers,
    ) {
        match kind {
            FlightComputeKind::TraceStats => {
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
            FlightComputeJob::TraceStats {
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
    use claims::{assert_lt, assert_none, assert_some_eq};

    fn fix(latitude: f64, longitude: f64, altitude: Option<f64>) -> GnssState {
        GnssState {
            position: Observation::new(Duration::ZERO, LatLon::from_degrees(latitude, longitude)),
            altitude: altitude.map(|meters| {
                Observation::new(
                    Duration::ZERO,
                    MslAltitude::new(Length::from_meters(meters)),
                )
            }),
            track: None,
            ground_speed: None,
        }
    }

    #[test]
    fn trace_stats_over_empty_trace() {
        let stats = trace_stats(&[]);
        assert_eq!(stats.fix_count, 0);
        assert_eq!(stats.distance, Length::ZERO);
        assert_none!(stats.max_altitude);
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
        assert_lt!((stats.distance.as_kilometers() - 222.6).abs(), 1.);
        assert_some_eq!(
            stats.max_altitude,
            MslAltitude::new(Length::from_meters(1500.))
        );
    }

    #[test]
    fn input_reports_observation_time() {
        let position = fix(50., 6., Some(1000.));
        let observation = Observation::new(
            position.position.observed_at,
            GnssUpdate {
                position: position.position.value,
                altitude: position.altitude.map(|altitude| altitude.value),
                track: position.track.map(|track| track.value),
                ground_speed: position.ground_speed.map(|speed| speed.value),
            },
        );

        assert_some_eq!(
            FlightInput::Gnss(Sourced::simulator(observation)).observed_at(),
            position.position.observed_at
        );
        assert_none!(FlightInput::ClearTrace.observed_at());
    }
}
