use crate::UtcInstant;
use crate::climb::Velocity;
use updraft_geo::LatLon;
use updraft_units::{MslAltitude, Speed};

/// A fix this much later than the last sample starts a new recording.
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
    pub netto: Option<Speed>,
    pub relative_vario: Option<Speed>,
    /// The velocity of the air mass.
    pub wind: Option<Velocity>,
}

#[derive(Clone, Copy, Debug)]
pub struct RecordedFix {
    pub starts_recording: bool,
    pub recording_start: UtcInstant,
    pub utc: UtcInstant,
    pub position: LatLon,
}

/// Applies the recording rules to the fixes of the selected source.
#[derive(Debug, Default)]
pub struct FlightRecorder {
    /// The UTC of the first and the last sample of the current recording.
    recording: Option<(UtcInstant, UtcInstant)>,
    pending: Vec<RecordedFix>,
}

impl FlightRecorder {
    /// Returns whether the fix starts a new recording, or `None` when the
    /// recorder drops the fix.
    pub fn observe(&mut self, utc: UtcInstant, position: LatLon) -> Option<bool> {
        let recording_start = match self.recording {
            Some((start, last)) => {
                let gap = utc.unix_milliseconds() - last.unix_milliseconds();
                if (-MAX_BACKWARD_MILLISECONDS..=0).contains(&gap) {
                    return None;
                }
                match (1..=MAX_GAP_MILLISECONDS).contains(&gap) {
                    true => start,
                    false => utc,
                }
            }
            None => utc,
        };
        let starts_recording = recording_start == utc;
        self.recording = Some((recording_start, utc));
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
        self.recording = Some((start, last));
        true
    }

    pub fn take_pending(&mut self) -> Vec<RecordedFix> {
        std::mem::take(&mut self.pending)
    }
}
