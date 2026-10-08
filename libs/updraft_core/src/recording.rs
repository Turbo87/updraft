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
    pub utc: UtcInstant,
    pub position: LatLon,
}

/// Applies the recording rules to the fixes of the selected source.
#[derive(Debug, Default)]
pub struct FlightRecorder {
    last_utc: Option<UtcInstant>,
    pending: Vec<RecordedFix>,
}

impl FlightRecorder {
    /// Returns whether the fix starts a new recording.
    pub fn observe(&mut self, utc: UtcInstant, position: LatLon) -> bool {
        let gap = self
            .last_utc
            .map(|last| utc.unix_milliseconds() - last.unix_milliseconds());
        let starts_recording = match gap {
            Some(gap) if (-MAX_BACKWARD_MILLISECONDS..=0).contains(&gap) => return false,
            Some(gap) => !(1..=MAX_GAP_MILLISECONDS).contains(&gap),
            None => true,
        };
        self.last_utc = Some(utc);
        self.pending.push(RecordedFix {
            starts_recording,
            utc,
            position,
        });
        starts_recording
    }

    /// Continues the recording after its last sample. Returns `false` when
    /// the recording ended more than 3 h before `utc`.
    pub fn restore(&mut self, last: UtcInstant, utc: UtcInstant) -> bool {
        if utc.unix_milliseconds() - last.unix_milliseconds() > MAX_GAP_MILLISECONDS {
            return false;
        }
        self.last_utc = Some(last);
        true
    }

    pub fn take_pending(&mut self) -> Vec<RecordedFix> {
        std::mem::take(&mut self.pending)
    }
}
