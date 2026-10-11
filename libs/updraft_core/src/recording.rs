use crate::UtcInstant;
use crate::climb::Velocity;
use serde::Serialize;
use updraft_geo::LatLon;
use updraft_units::{MslAltitude, Speed};

/// A fix this much later than the last sample asks the pilot whether a new
/// flight starts. More shell time since the last sample starts a new
/// recording.
const MAX_GAP_MILLISECONDS: i64 = 3 * 60 * 60 * 1000;

/// A fix this much earlier than the last sample asks the pilot whether a
/// new flight starts. A fix that is less early is dropped silently.
const MAX_BACKWARD_MILLISECONDS: i64 = 30 * 1000;

/// One recorded fix of the selected source with the fused values for it.
///
/// A fused value is `None` when it is unavailable or stale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub utc: UtcInstant,
    pub position: LatLon,
    /// The altitude that the altitude infobox shows.
    pub altitude_msl: Option<MslAltitude>,
    pub vario: Option<Speed>,
    pub vertical_speed: Option<Speed>,
    pub netto: Option<Speed>,
    pub relative_vario: Option<Speed>,
    pub estimated_netto: Option<Speed>,
    pub estimated_relative_vario: Option<Speed>,
    /// The velocity of the air mass.
    pub wind: Option<Velocity>,
}

/// One sample that the shell stored, with the terrain elevation that the
/// shell stored with it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoredSample {
    pub sample: Sample,
    pub terrain_elevation_meters: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
pub struct RecordedFix {
    pub starts_recording: bool,
    pub recording_start: UtcInstant,
    pub utc: UtcInstant,
    pub position: LatLon,
}

/// The open question to the pilot whether a new flight starts, with its
/// cause.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "cause",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FlightBoundaryQuestion {
    /// The fix UTC moved by this step from the last sample. A negative step
    /// moves back.
    FixUtcJump {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        milliseconds: i64,
    },
    /// A confirmed position jump moved the position by this distance.
    PositionJump { meters: f64 },
}

/// What the recorder does with a fix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation {
    /// Whether the fix starts a new recording, or `None` when the recorder
    /// drops the fix.
    pub starts_recording: Option<bool>,
    /// The fix-UTC step from the last sample when the pilot must decide
    /// whether a new flight starts.
    pub fix_utc_jump: Option<i64>,
}

#[derive(Clone, Copy, Debug)]
struct Recording {
    start: UtcInstant,
    last: UtcInstant,
    /// The shell UTC of the last sample.
    last_shell_utc: Option<UtcInstant>,
}

/// Applies the recording rules to the fixes of the selected source.
#[derive(Debug, Default)]
pub struct FlightRecorder {
    recording: Option<Recording>,
    /// The next fix starts a new recording.
    new_flight: bool,
    /// The fix UTC jumped back, so fixes up to the last sample are dropped
    /// without a new question.
    backward_jump: bool,
    pending: Vec<RecordedFix>,
}

impl FlightRecorder {
    /// `shell_utc` is the shell UTC at the fix, or `None` before the shell
    /// supplies it.
    pub fn observe(
        &mut self,
        utc: UtcInstant,
        position: LatLon,
        shell_utc: Option<UtcInstant>,
    ) -> Observation {
        let continued = self
            .recording
            .filter(|recording| !self.new_flight && !recording.shell_gap_exceeded(shell_utc));
        let Some(recording) = continued else {
            self.new_flight = false;
            self.backward_jump = false;
            self.record(utc, utc, position, shell_utc);
            return Observation {
                starts_recording: Some(true),
                fix_utc_jump: None,
            };
        };
        let step = utc.unix_milliseconds() - recording.last.unix_milliseconds();
        if step <= 0 {
            let jump = step < -MAX_BACKWARD_MILLISECONDS && !self.backward_jump;
            self.backward_jump |= jump;
            return Observation {
                starts_recording: None,
                fix_utc_jump: jump.then_some(step),
            };
        }
        self.backward_jump = false;
        self.record(recording.start, utc, position, shell_utc);
        Observation {
            starts_recording: Some(false),
            fix_utc_jump: (step > MAX_GAP_MILLISECONDS).then_some(step),
        }
    }

    fn record(
        &mut self,
        recording_start: UtcInstant,
        utc: UtcInstant,
        position: LatLon,
        shell_utc: Option<UtcInstant>,
    ) {
        self.recording = Some(Recording {
            start: recording_start,
            last: utc,
            last_shell_utc: shell_utc,
        });
        self.pending.push(RecordedFix {
            starts_recording: recording_start == utc,
            recording_start,
            utc,
            position,
        });
    }

    /// Starts a new recording at the next fix.
    pub fn start_new_flight(&mut self) {
        self.new_flight = true;
    }

    /// Continues the recording after its last sample. Returns `false` when
    /// the shell stored the last sample more than 3 h before `utc`.
    ///
    /// Both values come from the shell clock, so the fix UTC of the source
    /// does not affect the result.
    pub fn restore(
        &mut self,
        start: UtcInstant,
        last: UtcInstant,
        stored_utc: UtcInstant,
        utc: UtcInstant,
    ) -> bool {
        if utc.unix_milliseconds() - stored_utc.unix_milliseconds() > MAX_GAP_MILLISECONDS {
            return false;
        }
        self.recording = Some(Recording {
            start,
            last,
            last_shell_utc: Some(stored_utc),
        });
        true
    }

    pub fn take_pending(&mut self) -> Vec<RecordedFix> {
        std::mem::take(&mut self.pending)
    }
}

impl Recording {
    fn shell_gap_exceeded(&self, shell_utc: Option<UtcInstant>) -> bool {
        let Some((last, now)) = self.last_shell_utc.zip(shell_utc) else {
            return false;
        };
        now.unix_milliseconds() - last.unix_milliseconds() > MAX_GAP_MILLISECONDS
    }
}
