use bytes::Bytes;
use indicatif::ProgressStyle;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{Instrument as _, Span, info, info_span};
use tracing_indicatif::span_ext::IndicatifSpanExt as _;
use updraft_nmea::{Message, Step, Time, parse};

const DAY_MILLIS: u64 = 24 * 60 * 60 * 1_000;
const HALF_DAY_MILLIS: u64 = 12 * 60 * 60 * 1_000;

#[derive(Clone, Debug)]
pub struct ReplayEvent {
    at: Duration,
    payload: Bytes,
}

impl ReplayEvent {
    pub fn new(at: Duration, payload: Bytes) -> Self {
        Self { at, payload }
    }

    pub fn at(&self) -> Duration {
        self.at
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }
}

#[derive(Clone, Debug)]
pub struct Replay {
    events: Vec<ReplayEvent>,
    duration: Duration,
    timestamp_regression: bool,
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ReplayError {
    #[error("NMEA file has no valid RMC or GGA timestamp")]
    MissingTimestamp,
    #[error("skip {skip:?} exceeds replay duration {duration:?}")]
    SkipExceedsDuration { skip: Duration, duration: Duration },
}

impl Replay {
    pub fn from_nmea(bytes: Vec<u8>) -> Result<Self, ReplayError> {
        let source = Bytes::from(bytes);
        let source_len = source.len();
        let mut input = source.as_ref();
        let mut events = Vec::new();
        let mut first_absolute = None;
        let mut event_time = None;
        let mut range_start = 0;
        let mut previous_time = None;
        let mut day_offset = 0;
        let mut current_time = None;
        let mut timestamp_regression = false;

        // Build events while the parser advances through the source.
        loop {
            let input_offset = source_len - input.len();
            // The parser consumes leading line breaks. Record the frame start first.
            let frame_offset = input_offset
                + input
                    .iter()
                    .position(|byte| !matches!(byte, b'\r' | b'\n'))
                    .unwrap_or(input.len());

            let time = match parse(&mut input) {
                Step::Frame(Message::Rmc(rmc)) => rmc.utc_time,
                Step::Frame(Message::Gga(gga)) => gga.utc_time,
                Step::Incomplete => break,
                _ => None,
            };

            if let Some(time) = time {
                let absolute = schedule_millis(
                    time,
                    &mut previous_time,
                    &mut day_offset,
                    &mut current_time,
                    &mut timestamp_regression,
                );
                let first_absolute = *first_absolute.get_or_insert(absolute);
                let anchor_time = Duration::from_millis(absolute - first_absolute);

                match event_time {
                    None => event_time = Some(anchor_time),
                    Some(current) if anchor_time > current => {
                        events.push(ReplayEvent::new(
                            current,
                            source.slice(range_start..frame_offset),
                        ));
                        range_start = frame_offset;
                        event_time = Some(anchor_time);
                    }
                    // Equal or clamped anchors remain in the current event.
                    Some(_) => {}
                }
            }
        }

        let duration = event_time.ok_or(ReplayError::MissingTimestamp)?;
        events.push(ReplayEvent::new(
            duration,
            source.slice(range_start..source_len),
        ));

        Ok(Self {
            events,
            duration,
            timestamp_regression,
        })
    }

    pub fn events(&self) -> &[ReplayEvent] {
        &self.events
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn has_timestamp_regression(&self) -> bool {
        self.timestamp_regression
    }

    pub fn events_from(&self, skip: Duration) -> Result<&[ReplayEvent], ReplayError> {
        if skip > self.duration {
            return Err(ReplayError::SkipExceedsDuration {
                skip,
                duration: self.duration,
            });
        }

        let first = self.events.partition_point(|event| event.at < skip);
        Ok(&self.events[first..])
    }

    pub async fn play(
        &self,
        sender: broadcast::Sender<Bytes>,
        skip: Duration,
        loop_playback: bool,
    ) -> Result<(), ReplayError> {
        let mut first_pass = true;

        loop {
            let events = if first_pass {
                self.events_from(skip)?
            } else {
                self.events()
            };
            let first_time = events[0].at();
            let span = replay_span(self.duration(), first_time);

            async {
                let mut previous = first_time;
                for event in events {
                    tokio::time::sleep(event.at() - previous).await;
                    let payload = event.payload().clone();
                    info!("{}", String::from_utf8_lossy(payload.as_ref()).trim_end());
                    let _ = sender.send(payload);
                    set_progress(&Span::current(), event.at(), self.duration());
                    previous = event.at();
                }
            }
            .instrument(span)
            .await;

            if !loop_playback {
                return Ok(());
            }
            first_pass = false;
        }
    }
}

fn schedule_millis(
    time: Time,
    previous_time: &mut Option<u64>,
    day_offset: &mut u64,
    current_time: &mut Option<u64>,
    timestamp_regression: &mut bool,
) -> u64 {
    let time = u64::from(time.milliseconds_since_midnight());

    if let Some(previous) = *previous_time
        && previous > time
        && previous - time > HALF_DAY_MILLIS
    {
        *day_offset += DAY_MILLIS;
    }

    let candidate = *day_offset + time;
    let scheduled = match *current_time {
        Some(current) if candidate < current => {
            *timestamp_regression = true;
            current
        }
        _ => candidate,
    };

    *previous_time = Some(time);
    *current_time = Some(scheduled);
    scheduled
}

fn replay_span(duration: Duration, position: Duration) -> Span {
    let span = info_span!("replay");
    let style = ProgressStyle::default_bar()
        .template("{msg} {bar:40.cyan/blue}")
        .expect("valid replay progress template");
    span.pb_set_style(&style);
    span.pb_set_length(duration.as_secs());
    set_progress(&span, position, duration);
    span.pb_set_finish_message("Replay completed");
    span
}

fn set_progress(span: &Span, position: Duration, duration: Duration) {
    span.pb_set_position(position.as_secs());
    span.pb_set_message(&format!(
        "{} / {}",
        display_duration(position),
        display_duration(duration),
    ));
}

fn display_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3_600;
    let minutes = total_seconds / 60 % 60;
    let seconds = total_seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_ok, assert_some_eq};
    use std::assert_matches;

    const BASIC: &[u8] = include_bytes!("../../../testdata/nmea/basic.nmea");

    #[test]
    fn builds_a_byte_preserving_schedule_and_applies_skip() {
        let replay = assert_ok!(Replay::from_nmea(BASIC.to_vec()));

        assert_eq!(
            replay
                .events()
                .iter()
                .map(ReplayEvent::at)
                .collect::<Vec<_>>(),
            [
                Duration::ZERO,
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(3),
            ]
        );
        assert_eq!(replay.duration(), Duration::from_secs(3));
        assert!(!replay.has_timestamp_regression());

        let reconstructed = replay
            .events()
            .iter()
            .flat_map(|event| event.payload())
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(reconstructed.as_slice(), BASIC);

        let skipped = assert_ok!(replay.events_from(Duration::from_secs(2)));
        assert_some_eq!(skipped.first().map(ReplayEvent::at), Duration::from_secs(2));
    }

    #[test]
    fn rejects_a_file_without_a_timestamp() {
        let error = Replay::from_nmea(b"$PGRMZ,1000,f,3\r\n".to_vec());
        assert_matches!(error, Err(ReplayError::MissingTimestamp));
    }
}
