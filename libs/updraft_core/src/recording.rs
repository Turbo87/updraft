use crate::UtcInstant;
use crate::climb::Velocity;
use updraft_geo::LatLon;
use updraft_units::{MslAltitude, Speed};

/// A fix this much later than the last sample starts a new recording. The
/// same limit applies to the shell time since the last sample.
const MAX_GAP_MILLISECONDS: i64 = 3 * 60 * 60 * 1000;

/// A fix this much earlier than the last sample starts a new recording.
/// A fix that is less early is dropped.
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
    pending: Vec<RecordedFix>,
}

impl FlightRecorder {
    /// Returns whether the fix starts a new recording, or `None` when the
    /// recorder drops the fix.
    ///
    /// `shell_utc` is the shell UTC at the fix, or `None` before the shell
    /// supplies it.
    pub fn observe(
        &mut self,
        utc: UtcInstant,
        position: LatLon,
        shell_utc: Option<UtcInstant>,
    ) -> Option<bool> {
        let recording_start = match self.recording {
            Some(recording) if !recording.shell_gap_exceeded(shell_utc) => {
                let gap = utc.unix_milliseconds() - recording.last.unix_milliseconds();
                if (-MAX_BACKWARD_MILLISECONDS..=0).contains(&gap) {
                    return None;
                }
                match (1..=MAX_GAP_MILLISECONDS).contains(&gap) {
                    true => recording.start,
                    false => utc,
                }
            }
            _ => utc,
        };
        let starts_recording = recording_start == utc;
        self.recording = Some(Recording {
            start: recording_start,
            last: utc,
            last_shell_utc: shell_utc,
        });
        self.pending.push(RecordedFix {
            starts_recording,
            recording_start,
            utc,
            position,
        });
        Some(starts_recording)
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
