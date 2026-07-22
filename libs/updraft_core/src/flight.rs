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

const FLIGHT_SIGNAL_FRESHNESS: Duration = Duration::from_secs(3);

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

impl GnssState {
    fn data_at(self, clock_time: Duration) -> GnssData {
        GnssData {
            position: observation_availability(Some(self.position), clock_time),
            altitude: observation_availability(self.altitude, clock_time),
            track: observation_availability(self.track, clock_time),
            ground_speed: observation_availability(self.ground_speed, clock_time),
        }
    }

    fn trace_point_at(self, clock_time: Duration) -> TracePoint {
        TracePoint {
            position: self.position.value,
            altitude: self
                .altitude
                .filter(|altitude| is_fresh(altitude.observed_at, clock_time))
                .map(|altitude| altitude.value),
        }
    }

    fn freshness_deadline_after(self, clock_time: Duration) -> Option<Duration> {
        [
            Some(self.position.observed_at),
            self.altitude.map(|altitude| altitude.observed_at),
            self.track.map(|track| track.observed_at),
            self.ground_speed.map(|speed| speed.observed_at),
        ]
        .into_iter()
        .flatten()
        .map(|observed_at| observed_at.saturating_add(FLIGHT_SIGNAL_FRESHNESS))
        .filter(|deadline| *deadline > clock_time)
        .min()
    }
}

/// The availability of a published flight value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Availability<T> {
    /// No usable value has been selected.
    #[default]
    Unavailable,
    /// The selected value is fresh.
    Current(T),
    /// The selected value is stale.
    LastKnown(T),
}

/// Published data from the selected GNSS source.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GnssData {
    pub position: Availability<LatLon>,
    /// Mean-sea-level GNSS altitude.
    pub altitude: Availability<MslAltitude>,
    /// Track over ground.
    pub track: Availability<Angle>,
    pub ground_speed: Availability<Speed>,
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

/// One accepted horizontal position and its fresh GNSS altitude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TracePoint {
    pub position: LatLon,
    pub altitude: Option<MslAltitude>,
}

/// A recorded event or request owned by the flight domain.
#[derive(Clone, Debug, PartialEq)]
pub enum FlightInput {
    /// Clears the flown trace and its statistics.
    ClearTrace,
    /// Replaces external-device preference with `Internal` as the final live fallback.
    SetExternalDeviceOrder(Vec<DeviceId>),
    /// A GNSS position update.
    Gnss(Sourced<Observation<GnssUpdate>>),
    /// A standard-pressure altitude observation.
    PressureAltitude(Sourced<Observation<PressureAltitude>>),
}

impl FlightInput {
    pub(crate) fn observed_at(&self) -> Option<Duration> {
        match self {
            Self::Gnss(gnss) => Some(gnss.value.observed_at),
            Self::PressureAltitude(altitude) => Some(altitude.value.observed_at),
            _ => None,
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
    /// The selected GNSS data.
    Gnss(GnssData),
    /// The selected standard-pressure altitude and its availability.
    PressureAltitude(Availability<PressureAltitude>),
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
        fixes: Vec<TracePoint>,
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
    pub gnss: GnssData,
    pub pressure_altitude: Availability<PressureAltitude>,
    pub trace_stats: Option<TraceStats>,
}

/// Computes trace statistics using a WGS84 geodesic solve for each segment.
///
/// Its cost grows with the trace, so it runs on a compute worker.
pub(crate) fn trace_stats(fixes: &[TracePoint]) -> TraceStats {
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
    external_device_order: Vec<DeviceId>,
    source_states: HashMap<SourceId, SourceState>,
    selected_gnss_source: Option<SourceId>,
    selected_pressure_altitude_source: Option<SourceId>,
    trace: Vec<TracePoint>,
    trace_stats: Option<TraceStats>,
    stats_job: ComputeSlot,
    stats_started_at: Option<Duration>,
}

impl Flight {
    pub(crate) fn new(config: FlightConfig) -> Self {
        Self {
            stats_interval: config.trace_stats_interval,
            external_device_order: Vec::new(),
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
            FlightInput::SetExternalDeviceOrder(order) => {
                self.external_device_order = order;
                self.reselect_live_sources(clock_time, timers, update);
            }
            FlightInput::Gnss(gnss) => self.observe_gnss(gnss, clock_time, timers, update),
            FlightInput::PressureAltitude(altitude) => {
                self.observe_pressure_altitude(altitude, clock_time, timers, update)
            }
        }
    }

    pub(crate) fn snapshot(&self, clock_time: Duration) -> FlightSnapshot {
        FlightSnapshot {
            gnss: self.gnss_data(clock_time),
            pressure_altitude: self.pressure_altitude_availability(clock_time),
            trace_stats: self.trace_stats(),
        }
    }

    fn gnss_for_source(&self, source: SourceId) -> Option<GnssState> {
        self.source_states.get(&source).and_then(|state| state.gnss)
    }

    fn pressure_altitude_for_source(
        &self,
        source: SourceId,
    ) -> Option<Observation<PressureAltitude>> {
        self.source_states
            .get(&source)
            .and_then(|state| state.pressure_altitude)
    }

    fn live_sources(&self) -> impl Iterator<Item = SourceId> + '_ {
        self.external_device_order
            .iter()
            .copied()
            .map(SourceId::External)
            .chain([SourceId::Internal])
    }

    fn gnss_data(&self, clock_time: Duration) -> GnssData {
        self.selected_gnss_source
            .and_then(|source| self.gnss_for_source(source))
            .map_or_else(GnssData::default, |gnss| gnss.data_at(clock_time))
    }

    fn pressure_altitude_availability(
        &self,
        clock_time: Duration,
    ) -> Availability<PressureAltitude> {
        match self
            .selected_pressure_altitude_source
            .and_then(|source| self.pressure_altitude_for_source(source))
        {
            Some(altitude) if is_fresh(altitude.observed_at, clock_time) => {
                Availability::Current(altitude.value)
            }
            Some(altitude) => Availability::LastKnown(altitude.value),
            None => Availability::Unavailable,
        }
    }

    fn gnss_freshness_deadline_after(&self, clock_time: Duration) -> Option<Duration> {
        self.selected_gnss_source
            .and_then(|source| self.gnss_for_source(source))
            .and_then(|gnss| gnss.freshness_deadline_after(clock_time))
    }

    fn pressure_altitude_freshness_deadline_after(&self, clock_time: Duration) -> Option<Duration> {
        self.selected_pressure_altitude_source
            .and_then(|source| self.pressure_altitude_for_source(source))
            .map(|altitude| altitude.observed_at.saturating_add(FLIGHT_SIGNAL_FRESHNESS))
            .filter(|deadline| *deadline > clock_time)
    }

    fn select_live_gnss_source(&self, clock_time: Duration) -> Option<SourceId> {
        self.live_sources()
            .find(|source| {
                self.gnss_for_source(*source)
                    .is_some_and(|gnss| is_fresh(gnss.position.observed_at, clock_time))
            })
            .or_else(|| {
                self.selected_gnss_source.filter(|selected| {
                    self.live_sources().any(|source| source == *selected)
                        && self.gnss_for_source(*selected).is_some()
                })
            })
            .or_else(|| {
                self.live_sources()
                    .find(|source| self.gnss_for_source(*source).is_some())
            })
    }

    fn select_live_pressure_altitude_source(&self, clock_time: Duration) -> Option<SourceId> {
        self.live_sources()
            .find(|source| {
                self.pressure_altitude_for_source(*source)
                    .is_some_and(|altitude| is_fresh(altitude.observed_at, clock_time))
            })
            .or_else(|| {
                self.selected_pressure_altitude_source.filter(|selected| {
                    self.live_sources().any(|source| source == *selected)
                        && self.pressure_altitude_for_source(*selected).is_some()
                })
            })
            .or_else(|| {
                self.live_sources()
                    .find(|source| self.pressure_altitude_for_source(*source).is_some())
            })
    }

    pub(crate) fn observe_gnss(
        &mut self,
        observation: Sourced<Observation<GnssUpdate>>,
        clock_time: Duration,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        let previous = self.gnss_data(clock_time);
        let source = observation.source;
        let observation = observation.value;
        let Some(gnss) = self
            .source_states
            .entry(source)
            .or_default()
            .observe_gnss(observation)
        else {
            return;
        };
        if matches!(source, SourceId::Simulator | SourceId::Replay) {
            self.selected_gnss_source = Some(source);
        } else if !matches!(
            self.selected_gnss_source,
            Some(SourceId::Simulator | SourceId::Replay)
        ) {
            self.selected_gnss_source = self.select_live_gnss_source(clock_time);
        }
        let next = self.gnss_data(clock_time);
        if next != previous {
            update
                .changes
                .push(AppChange::Flight(FlightChange::Gnss(next)));
        }
        self.schedule_signal_freshness(clock_time, timers);
        if self.selected_gnss_source != Some(source)
            || !is_fresh(gnss.position.observed_at, clock_time)
        {
            return;
        }

        self.trace.push(gnss.trace_point_at(clock_time));
        self.stats_job.request();
        self.schedule_stats(clock_time, timers);
    }

    fn observe_pressure_altitude(
        &mut self,
        observation: Sourced<Observation<PressureAltitude>>,
        clock_time: Duration,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        let previous = self.pressure_altitude_availability(clock_time);
        let source = observation.source;
        let observation = observation.value;
        if !self
            .source_states
            .entry(source)
            .or_default()
            .observe_pressure_altitude(observation)
        {
            return;
        }
        if matches!(source, SourceId::Simulator | SourceId::Replay) {
            self.selected_pressure_altitude_source = Some(source);
        } else if !matches!(
            self.selected_pressure_altitude_source,
            Some(SourceId::Simulator | SourceId::Replay)
        ) {
            self.selected_pressure_altitude_source =
                self.select_live_pressure_altitude_source(clock_time);
        }
        let next = self.pressure_altitude_availability(clock_time);
        if next != previous {
            update
                .changes
                .push(AppChange::Flight(FlightChange::PressureAltitude(next)));
        }
        self.schedule_signal_freshness(clock_time, timers);
    }

    fn reselect_live_sources(
        &mut self,
        clock_time: Duration,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        let previous_gnss = self.gnss_data(clock_time);
        let previous_pressure_altitude = self.pressure_altitude_availability(clock_time);
        let external_device_order = &self.external_device_order;
        self.source_states.retain(|source, _| match source {
            SourceId::External(device_id) => external_device_order.contains(device_id),
            _ => true,
        });
        self.reselect_gnss(clock_time);
        self.reselect_pressure_altitude(clock_time);
        let gnss = self.gnss_data(clock_time);
        if gnss != previous_gnss {
            update
                .changes
                .push(AppChange::Flight(FlightChange::Gnss(gnss)));
        }
        let pressure_altitude = self.pressure_altitude_availability(clock_time);
        if pressure_altitude != previous_pressure_altitude {
            update
                .changes
                .push(AppChange::Flight(FlightChange::PressureAltitude(
                    pressure_altitude,
                )));
        }
        self.schedule_signal_freshness(clock_time, timers);
    }

    fn reselect_gnss(&mut self, clock_time: Duration) {
        if !matches!(
            self.selected_gnss_source,
            Some(SourceId::Simulator | SourceId::Replay)
        ) {
            self.selected_gnss_source = self.select_live_gnss_source(clock_time);
        }
    }

    fn reselect_pressure_altitude(&mut self, clock_time: Duration) {
        if !matches!(
            self.selected_pressure_altitude_source,
            Some(SourceId::Simulator | SourceId::Replay)
        ) {
            self.selected_pressure_altitude_source =
                self.select_live_pressure_altitude_source(clock_time);
        }
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

    pub(crate) fn timer(
        &mut self,
        timer: Timer,
        scheduled_at: Duration,
        clock_time: Duration,
        timers: &mut Timers,
        update: &mut Update,
    ) {
        match timer {
            Timer::FlightSignalFreshness => {
                // Compare across the scheduled boundary even when the runtime
                // delivers the timer late.
                let previous_time = scheduled_at.saturating_sub(Duration::from_nanos(1));
                let previous_gnss = self.gnss_data(previous_time);
                let previous_pressure_altitude = self.pressure_altitude_availability(previous_time);

                self.reselect_gnss(clock_time);
                self.reselect_pressure_altitude(clock_time);

                let gnss = self.gnss_data(clock_time);
                let gnss_changed = update
                    .changes
                    .iter()
                    .any(|change| matches!(change, AppChange::Flight(FlightChange::Gnss(_))));
                if !gnss_changed && gnss != previous_gnss {
                    update
                        .changes
                        .push(AppChange::Flight(FlightChange::Gnss(gnss)));
                }

                let pressure_altitude = self.pressure_altitude_availability(clock_time);
                let pressure_altitude_changed = update.changes.iter().any(|change| {
                    matches!(change, AppChange::Flight(FlightChange::PressureAltitude(_)))
                });
                if !pressure_altitude_changed && pressure_altitude != previous_pressure_altitude {
                    update
                        .changes
                        .push(AppChange::Flight(FlightChange::PressureAltitude(
                            pressure_altitude,
                        )));
                }
                self.schedule_signal_freshness(clock_time, timers);
            }
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

    fn schedule_signal_freshness(&self, clock_time: Duration, timers: &mut Timers) {
        if timers
            .deadline(Timer::FlightSignalFreshness)
            .is_some_and(|deadline| deadline <= clock_time)
        {
            return;
        }
        let gnss = self.gnss_freshness_deadline_after(clock_time);
        let pressure_altitude = self.pressure_altitude_freshness_deadline_after(clock_time);
        match [gnss, pressure_altitude].into_iter().flatten().min() {
            Some(at) => timers.schedule(Timer::FlightSignalFreshness, at),
            None => timers.cancel(Timer::FlightSignalFreshness),
        }
    }
}

fn observation_availability<T>(
    observation: Option<Observation<T>>,
    clock_time: Duration,
) -> Availability<T> {
    match observation {
        Some(observation) if is_fresh(observation.observed_at, clock_time) => {
            Availability::Current(observation.value)
        }
        Some(observation) => Availability::LastKnown(observation.value),
        None => Availability::Unavailable,
    }
}

fn is_fresh(observed_at: Duration, clock_time: Duration) -> bool {
    clock_time.saturating_sub(observed_at) < FLIGHT_SIGNAL_FRESHNESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_lt, assert_none, assert_some_eq};

    fn fix(latitude: f64, longitude: f64, altitude: Option<f64>) -> TracePoint {
        TracePoint {
            position: LatLon::from_degrees(latitude, longitude),
            altitude: altitude.map(|meters| MslAltitude::new(Length::from_meters(meters))),
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
        let observation = Observation::new(
            Duration::from_secs(1),
            GnssUpdate {
                position: LatLon::from_degrees(50., 6.),
                altitude: Some(MslAltitude::new(Length::from_meters(1000.))),
                track: None,
                ground_speed: None,
            },
        );

        assert_some_eq!(
            FlightInput::Gnss(Sourced::simulator(observation)).observed_at(),
            observation.observed_at
        );
        assert_none!(FlightInput::ClearTrace.observed_at());
        assert_none!(FlightInput::SetExternalDeviceOrder(Vec::new()).observed_at());
    }
}
