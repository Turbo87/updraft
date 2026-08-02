use bytes::Bytes;
use igc::records::{BRecord, FixValid, Record};
use indicatif::ProgressStyle;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{Instrument as _, Span, info, info_span};
use tracing_indicatif::span_ext::IndicatifSpanExt as _;
use updraft_geo::LatLon;
use updraft_nmea::{
    Date, EncodeError, Gga, GgaFixQuality, Message, Pgrmz, PgrmzFixDimension, Rmc, RmcStatus, Step,
    Talker, Time, parse,
};
use updraft_units::{EllipsoidAltitude, Length};

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
    #[error("IGC file has no usable B record")]
    MissingFix,
    #[error(transparent)]
    Encode(#[from] EncodeError),
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

    pub fn from_igc(input: &str) -> Result<Self, ReplayError> {
        let mut events = Vec::<ReplayEvent>::new();
        let mut first_absolute = None;
        let mut previous_time = None;
        let mut day_offset = 0;
        let mut current_time = None;
        let mut current_date = None;
        let mut timestamp_regression = false;

        for line in input.lines() {
            match Record::parse_line(line) {
                Ok(Record::H(header)) if header.mnemonic == "DTE" => {
                    current_date = current_date.or_else(|| {
                        header
                            .data
                            .split(',')
                            .next()
                            .and_then(|date| Date::parse_ddmmyy(date.as_bytes()))
                    });
                }
                Ok(Record::B(fix)) => {
                    let Some(time) = Time::from_hms_millis(
                        fix.timestamp.hours,
                        fix.timestamp.minutes,
                        fix.timestamp.seconds,
                        0,
                    ) else {
                        continue;
                    };

                    let previous_day_offset = day_offset;
                    let absolute = schedule_millis(
                        time,
                        &mut previous_time,
                        &mut day_offset,
                        &mut current_time,
                        &mut timestamp_regression,
                    );
                    if day_offset > previous_day_offset {
                        current_date = current_date.map(next_date);
                    }

                    let first_absolute = *first_absolute.get_or_insert(absolute);
                    let at = Duration::from_millis(absolute - first_absolute);
                    let payload = encode_fix(&fix, time, current_date)?;

                    if let Some(event) = events.last_mut()
                        && event.at == at
                    {
                        let mut combined = Vec::with_capacity(event.payload.len() + payload.len());
                        combined.extend_from_slice(&event.payload);
                        combined.extend_from_slice(&payload);
                        event.payload = Bytes::from(combined);
                    } else {
                        events.push(ReplayEvent::new(at, Bytes::from(payload)));
                    }
                }
                _ => {}
            }
        }

        let duration = events
            .last()
            .map(ReplayEvent::at)
            .ok_or(ReplayError::MissingFix)?;

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

fn encode_fix(fix: &BRecord<'_>, time: Time, date: Option<Date>) -> Result<Vec<u8>, EncodeError> {
    let position = LatLon::from_degrees(f64::from(fix.pos.lat), f64::from(fix.pos.lon));
    let (rmc_status, fix_quality, fix_dimension) = match fix.fix_valid {
        FixValid::Valid => (
            RmcStatus::Active,
            GgaFixQuality::Gps,
            PgrmzFixDimension::ThreeDimensional,
        ),
        FixValid::NavWarning => (
            RmcStatus::Void,
            GgaFixQuality::Invalid,
            PgrmzFixDimension::NoFix,
        ),
    };

    let rmc = Rmc {
        talker: Talker::Gps,
        utc_time: Some(time),
        status: rmc_status,
        position: Some(position),
        speed_over_ground: None,
        course_over_ground: None,
        date,
        magnetic_variation: None,
        mode: None,
    };

    let (altitude, geoid_separation) = if fix.gps_alt == 0 {
        (None, None)
    } else {
        let ellipsoid_altitude =
            EllipsoidAltitude::new(Length::from_meters(f64::from(fix.gps_alt)));
        (
            Some(updraft_egm96::ellipsoidal_to_msl(position, ellipsoid_altitude).into_inner()),
            Some(updraft_egm96::undulation(position)),
        )
    };
    let gga = Gga {
        talker: Talker::Gps,
        utc_time: Some(time),
        position: Some(position),
        fix_quality,
        satellites_used: None,
        hdop: None,
        altitude,
        geoid_separation,
        dgps_age: None,
        dgps_station: None,
    };

    let mut payload = Vec::new();
    payload.extend(Vec::<u8>::try_from(&rmc)?);
    payload.extend(Vec::<u8>::try_from(&gga)?);
    if fix.pressure_alt != 0 {
        payload.extend(Vec::<u8>::try_from(&Pgrmz {
            altitude: Some(Length::from_meters(f64::from(fix.pressure_alt))),
            fix_dimension,
        })?);
    }
    Ok(payload)
}

fn next_date(date: Date) -> Date {
    let days_in_month = match date.month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(date.year) => 29,
        2 => 28,
        _ => unreachable!("NMEA dates contain valid months"),
    };

    if date.day < days_in_month {
        Date::new(date.year, date.month, date.day + 1)
    } else if date.month < 12 {
        Date::new(date.year, date.month + 1, 1)
    } else {
        Date::new(date.year + 1, 1, 1)
    }
}

fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
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
    use claims::{assert_none, assert_ok, assert_some_eq};
    use std::assert_matches;

    const BASIC: &[u8] = include_bytes!("../../../testdata/nmea/basic.nmea");
    const BASIC_IGC: &str = include_str!("../../../testdata/igc/basic.igc");

    #[test]
    fn converts_igc_fixes_to_nmea_events() {
        let replay = assert_ok!(Replay::from_igc(BASIC_IGC));

        assert_eq!(
            replay
                .events()
                .iter()
                .map(ReplayEvent::at)
                .collect::<Vec<_>>(),
            [Duration::ZERO, Duration::from_secs(1)]
        );
        assert_eq!(replay.duration(), Duration::from_secs(1));

        let first_payload = assert_ok!(str::from_utf8(replay.events()[0].payload()));
        insta::assert_snapshot!(first_payload);
    }

    #[test]
    fn omits_zero_igc_altitudes() {
        let replay = assert_ok!(Replay::from_igc(
            "HFDTE040726,1\nB1347495200000N00700000EA0000000000\n"
        ));

        insta::assert_snapshot!(payload_text(&replay.events()[0]));
    }

    #[test]
    fn emits_rmc_without_an_igc_date() {
        let replay = assert_ok!(Replay::from_igc("B1347495200000N00700000EA0304801000\n"));

        assert_none!(first_rmc(&replay.events()[0]).date);
        insta::assert_snapshot!(payload_text(&replay.events()[0]));
    }

    #[test]
    fn maps_igc_navigation_warnings_to_invalid_fixes() {
        let replay = assert_ok!(Replay::from_igc(
            "HFDTE040726\nB1347495200000N00700000EV0304801000\n"
        ));

        insta::assert_snapshot!(payload_text(&replay.events()[0]));
    }

    #[test]
    fn advances_the_igc_date_after_midnight() {
        for (start_date, expected_date) in [
            ("280224", Date::new(2024, 2, 29)),
            ("290224", Date::new(2024, 3, 1)),
            ("311223", Date::new(2024, 1, 1)),
        ] {
            let input = format!(
                "HFDTE{start_date}\n\
                 B2359595200000N00700000EA0304801000\n\
                 B0000005200000N00700000EA0304801000\n"
            );
            let replay = assert_ok!(Replay::from_igc(&input));

            assert_eq!(
                replay
                    .events()
                    .iter()
                    .map(ReplayEvent::at)
                    .collect::<Vec<_>>(),
                [Duration::ZERO, Duration::from_secs(1)]
            );
            assert_some_eq!(first_rmc(&replay.events()[1]).date, expected_date);
        }
    }

    #[test]
    fn clamps_a_small_igc_timestamp_regression() {
        let replay = assert_ok!(Replay::from_igc(
            "B1347505200000N00700000EA0304801000\n\
             B1347495200000N00700000EA0304801000\n\
             B1347515200000N00700000EA0304801000\n"
        ));

        assert_eq!(
            replay
                .events()
                .iter()
                .map(ReplayEvent::at)
                .collect::<Vec<_>>(),
            [Duration::ZERO, Duration::from_secs(1)]
        );
        assert!(replay.has_timestamp_regression());
    }

    #[test]
    fn rejects_igc_without_a_usable_b_record() {
        let error = Replay::from_igc("HFDTE040726\nB2400005200000N00700000EA0304801000\n");

        assert_matches!(error, Err(ReplayError::MissingFix));
    }

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

    fn payload_text(event: &ReplayEvent) -> &str {
        assert_ok!(str::from_utf8(event.payload()))
    }

    fn first_rmc(event: &ReplayEvent) -> Rmc {
        let mut input = event.payload().as_ref();
        match parse(&mut input) {
            Step::Frame(Message::Rmc(rmc)) => rmc,
            step => panic!("expected RMC frame, got {step:?}"),
        }
    }
}
