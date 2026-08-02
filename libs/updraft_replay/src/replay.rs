use bytes::Bytes;
use igc::records::{BRecord, Extendable, Extension, FixValid, Record};
use indicatif::ProgressStyle;
use std::{str::FromStr, time::Duration};
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{Instrument as _, Span, info, info_span};
use tracing_indicatif::span_ext::IndicatifSpanExt as _;
use updraft_geo::LatLon;
use updraft_nmea::{
    Date, EncodeError, Gga, GgaFixQuality, Lxwp0, Lxwp1, Message, Pgrmz, PgrmzFixDimension, Plxvs,
    Rmc, RmcStatus, Step, Talker, Time, parse,
};
use updraft_units::{Angle, EllipsoidAltitude, Length, Speed};

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
    warnings: Vec<String>,
}

struct IgcEventBuilder {
    absolute: u64,
    at: Duration,
    payload: Vec<u8>,
    pressure_altitude: Option<Length>,
    lxwp0: Option<Lxwp0>,
    lxwp0_line: Option<usize>,
}

impl IgcEventBuilder {
    fn new(absolute: u64, at: Duration) -> Self {
        Self {
            absolute,
            at,
            payload: Vec::new(),
            pressure_altitude: None,
            lxwp0: None,
            lxwp0_line: None,
        }
    }

    fn add_fix(
        &mut self,
        fix: &BRecord<'_>,
        extensions: &[Extension<'_>],
        line_number: usize,
        warnings: &mut Vec<String>,
        time: Time,
        date: Option<Date>,
    ) {
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

        append_igc_sentence(
            &mut self.payload,
            Vec::<u8>::try_from(&Rmc {
                talker: Talker::Gps,
                utc_time: Some(time),
                status: rmc_status,
                position: Some(position),
                speed_over_ground: extension_value::<f64>(
                    fix,
                    extensions,
                    "GSP",
                    line_number,
                    warnings,
                )
                .map(|value| Speed::from_kilometers_per_hour(value / 100.0)),
                course_over_ground: extension_value(fix, extensions, "TRT", line_number, warnings)
                    .map(Angle::from_degrees),
                date,
                magnetic_variation: None,
                mode: None,
            }),
            "RMC",
            line_number,
            warnings,
        );

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
        append_igc_sentence(
            &mut self.payload,
            Vec::<u8>::try_from(&Gga {
                talker: Talker::Gps,
                utc_time: Some(time),
                position: Some(position),
                fix_quality,
                satellites_used: extension_value(fix, extensions, "SIU", line_number, warnings),
                hdop: None,
                altitude,
                geoid_separation,
                dgps_age: None,
                dgps_station: None,
            }),
            "GGA",
            line_number,
            warnings,
        );

        self.pressure_altitude =
            (fix.pressure_alt != 0).then(|| Length::from_meters(f64::from(fix.pressure_alt)));
        if let Some(pressure_altitude) = self.pressure_altitude {
            append_igc_sentence(
                &mut self.payload,
                Vec::<u8>::try_from(&Pgrmz {
                    altitude: Some(pressure_altitude),
                    fix_dimension,
                }),
                "PGRMZ",
                line_number,
                warnings,
            );
        }

        let true_airspeed = extension_value::<f64>(fix, extensions, "TAS", line_number, warnings)
            .map(|value| Speed::from_kilometers_per_hour(value / 100.0));
        let vario = extension_value::<f64>(fix, extensions, "VAT", line_number, warnings)
            .map(|value| Speed::from_meters_per_second(value / 100.0));
        if true_airspeed.is_some() || vario.is_some() || self.lxwp0.is_some() {
            let lxwp0 = self.lxwp0.get_or_insert_with(empty_lxwp0);
            lxwp0.true_airspeed = true_airspeed;
            lxwp0.pressure_altitude = self.pressure_altitude;
            lxwp0.vario_samples = vario.into_iter().collect();
            self.lxwp0_line = Some(line_number);
        }

        if let Some(outside_air_temperature) =
            extension_value::<f64>(fix, extensions, "OAT", line_number, warnings)
        {
            append_igc_sentence(
                &mut self.payload,
                Vec::<u8>::try_from(&Plxvs {
                    outside_air_temperature: Some(outside_air_temperature / 10.0),
                    mode: None,
                    supply_voltage: None,
                    igc_pressure_altitude: self.pressure_altitude,
                    flap_position: None,
                }),
                "PLXVS",
                line_number,
                warnings,
            );
        }
    }

    fn add_wind(
        &mut self,
        wind_direction: Option<Angle>,
        wind_speed: Option<Speed>,
        line_number: usize,
    ) {
        let lxwp0 = self.lxwp0.get_or_insert_with(empty_lxwp0);
        lxwp0.pressure_altitude = self.pressure_altitude;
        lxwp0.wind_direction = wind_direction;
        lxwp0.wind_speed = wind_speed;
        self.lxwp0_line = Some(line_number);
    }

    fn finish(mut self, warnings: &mut Vec<String>) -> ReplayEvent {
        if let Some(lxwp0) = self.lxwp0 {
            append_igc_sentence(
                &mut self.payload,
                Vec::<u8>::try_from(&lxwp0),
                "LXWP0",
                self.lxwp0_line.expect("LXWP0 source line exists"),
                warnings,
            );
        }
        ReplayEvent::new(self.at, Bytes::from(self.payload))
    }
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
            warnings: timestamp_regression
                .then(|| "NMEA replay timestamps moved backward".to_owned())
                .into_iter()
                .collect(),
        })
    }

    pub fn from_igc(input: &str) -> Result<Self, ReplayError> {
        let mut events = Vec::<ReplayEvent>::new();
        let mut event_builder = None;
        let mut first_absolute = None;
        let mut previous_time = None;
        let mut day_offset = 0;
        let mut current_time = None;
        let mut current_date = None;
        let mut fix_extensions = Vec::new();
        let mut wind_extensions = Vec::new();
        let mut identification = empty_lxwp1();
        let mut has_fix = false;
        let mut timestamp_regression = false;
        let mut warnings = Vec::new();

        for (line_index, line) in input.lines().enumerate() {
            let line_number = line_index + 1;
            match Record::parse_line(line) {
                Ok(Record::A(record)) => {
                    identification.serial = nonempty_text(record.unique_id);
                    omit_invalid_igc_identification_field(
                        &mut identification,
                        "A",
                        line_number,
                        &mut warnings,
                    );
                }
                Ok(Record::H(header)) => match header.mnemonic {
                    "DTE" => {
                        let date = header
                            .data
                            .split(',')
                            .next()
                            .and_then(|date| Date::parse_ddmmyy(date.as_bytes()));
                        if date.is_none() {
                            warnings.push(format!("IGC line {line_number}: invalid DTE field"));
                        }
                        current_date = current_date.or(date);
                    }
                    "FTY" => {
                        identification.product =
                            header.data.rsplit(',').next().and_then(nonempty_text);
                        omit_invalid_igc_identification_field(
                            &mut identification,
                            "FTY",
                            line_number,
                            &mut warnings,
                        );
                    }
                    "RFW" => {
                        identification.software_version = nonempty_text(header.data);
                        omit_invalid_igc_identification_field(
                            &mut identification,
                            "RFW",
                            line_number,
                            &mut warnings,
                        );
                    }
                    "RHW" => {
                        identification.hardware_version = nonempty_text(header.data);
                        omit_invalid_igc_identification_field(
                            &mut identification,
                            "RHW",
                            line_number,
                            &mut warnings,
                        );
                    }
                    _ => {}
                },
                Ok(Record::I(definition)) => fix_extensions = definition.0.extensions,
                Ok(Record::J(definition)) => wind_extensions = definition.0.extensions,
                Ok(Record::B(fix)) => {
                    let Some(time) = Time::from_hms_millis(
                        fix.timestamp.hours,
                        fix.timestamp.minutes,
                        fix.timestamp.seconds,
                        0,
                    ) else {
                        warnings.push(format!("IGC line {line_number}: invalid B record"));
                        continue;
                    };
                    has_fix = true;

                    let previous_day_offset = day_offset;
                    let had_timestamp_regression = timestamp_regression;
                    let scheduled = schedule_millis(
                        time,
                        &mut previous_time,
                        &mut day_offset,
                        &mut current_time,
                        &mut timestamp_regression,
                    );
                    if !had_timestamp_regression && timestamp_regression {
                        warnings.push(format!("IGC line {line_number}: timestamp moved backward"));
                    }
                    if day_offset > previous_day_offset {
                        current_date = current_date.map(next_date);
                    }

                    let absolute = day_offset + u64::from(time.milliseconds_since_midnight());
                    let first_absolute = *first_absolute.get_or_insert(scheduled);
                    let at = Duration::from_millis(scheduled - first_absolute);
                    current_igc_event_builder(
                        &mut events,
                        &mut event_builder,
                        absolute,
                        at,
                        &mut warnings,
                    )
                    .add_fix(
                        &fix,
                        &fix_extensions,
                        line_number,
                        &mut warnings,
                        time,
                        current_date,
                    );
                }
                Ok(Record::K(record)) => {
                    let wind_direction = extension_value::<f64>(
                        &record,
                        &wind_extensions,
                        "WDI",
                        line_number,
                        &mut warnings,
                    )
                    .map(Angle::from_degrees);
                    let wind_speed = extension_value::<f64>(
                        &record,
                        &wind_extensions,
                        "WSP",
                        line_number,
                        &mut warnings,
                    )
                    .map(|value| Speed::from_kilometers_per_hour(value / 100.0));
                    if wind_direction.is_none() && wind_speed.is_none() {
                        continue;
                    }
                    let Some(time) = Time::from_hms_millis(
                        record.time.hours,
                        record.time.minutes,
                        record.time.seconds,
                        0,
                    ) else {
                        warnings.push(format!("IGC line {line_number}: invalid K record"));
                        continue;
                    };

                    let previous_day_offset = day_offset;
                    let had_timestamp_regression = timestamp_regression;
                    let scheduled = schedule_millis(
                        time,
                        &mut previous_time,
                        &mut day_offset,
                        &mut current_time,
                        &mut timestamp_regression,
                    );
                    if !had_timestamp_regression && timestamp_regression {
                        warnings.push(format!("IGC line {line_number}: timestamp moved backward"));
                    }
                    if day_offset > previous_day_offset {
                        current_date = current_date.map(next_date);
                    }

                    let absolute = day_offset + u64::from(time.milliseconds_since_midnight());
                    let first_absolute = *first_absolute.get_or_insert(scheduled);
                    let at = Duration::from_millis(scheduled - first_absolute);
                    current_igc_event_builder(
                        &mut events,
                        &mut event_builder,
                        absolute,
                        at,
                        &mut warnings,
                    )
                    .add_wind(wind_direction, wind_speed, line_number);
                }
                Err(_) => {
                    if let Some(record_type) = mapped_igc_record_type(line) {
                        warnings.push(format!(
                            "IGC line {line_number}: invalid {record_type} record"
                        ));
                    }
                }
                _ => {}
            }
        }

        if !has_fix {
            return Err(ReplayError::MissingFix);
        }
        if let Some(event) = event_builder {
            push_or_merge_igc_event(&mut events, event.finish(&mut warnings));
        }
        let duration = events
            .last()
            .map(ReplayEvent::at)
            .ok_or(ReplayError::MissingFix)?;
        insert_periodic_igc_identification(
            &mut events,
            Bytes::from(Vec::<u8>::try_from(&identification)?),
            duration,
        );

        Ok(Self {
            events,
            duration,
            warnings,
        })
    }

    pub fn events(&self) -> &[ReplayEvent] {
        &self.events
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
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

fn current_igc_event_builder<'a>(
    events: &mut Vec<ReplayEvent>,
    current: &'a mut Option<IgcEventBuilder>,
    absolute: u64,
    at: Duration,
    warnings: &mut Vec<String>,
) -> &'a mut IgcEventBuilder {
    if let Some(event) = current.take_if(|event| event.absolute != absolute) {
        push_or_merge_igc_event(events, event.finish(warnings));
    }

    current.get_or_insert_with(|| IgcEventBuilder::new(absolute, at))
}

fn push_or_merge_igc_event(events: &mut Vec<ReplayEvent>, event: ReplayEvent) {
    if let Some(previous) = events.last_mut()
        && previous.at == event.at
    {
        let mut combined = Vec::with_capacity(previous.payload.len() + event.payload.len());
        combined.extend_from_slice(&previous.payload);
        combined.extend_from_slice(&event.payload);
        previous.payload = Bytes::from(combined);
    } else {
        events.push(event);
    }
}

fn append_igc_sentence(
    payload: &mut Vec<u8>,
    sentence: Result<Vec<u8>, EncodeError>,
    sentence_name: &str,
    line_number: usize,
    warnings: &mut Vec<String>,
) {
    match sentence {
        Ok(sentence) => payload.extend(sentence),
        Err(_) => warnings.push(format!(
            "IGC line {line_number}: {sentence_name} sentence cannot be encoded"
        )),
    }
}

fn insert_periodic_igc_identification(
    events: &mut Vec<ReplayEvent>,
    identification: Bytes,
    duration: Duration,
) {
    for seconds in (0..=duration.as_secs()).step_by(60) {
        let at = Duration::from_secs(seconds);
        match events.binary_search_by_key(&at, ReplayEvent::at) {
            Ok(index) => {
                let event = &mut events[index];
                let mut payload = Vec::with_capacity(identification.len() + event.payload.len());
                payload.extend_from_slice(&identification);
                payload.extend_from_slice(&event.payload);
                event.payload = Bytes::from(payload);
            }
            Err(index) => events.insert(index, ReplayEvent::new(at, identification.clone())),
        }
    }
}

fn mapped_igc_record_type(line: &str) -> Option<char> {
    line.as_bytes()
        .first()
        .copied()
        .filter(|record_type| matches!(record_type, b'A' | b'H' | b'I' | b'J' | b'B' | b'K'))
        .map(char::from)
}

fn empty_lxwp0() -> Lxwp0 {
    Lxwp0 {
        logger_running: None,
        true_airspeed: None,
        pressure_altitude: None,
        vario_samples: Vec::new(),
        heading: None,
        wind_direction: None,
        wind_speed: None,
    }
}

fn empty_lxwp1() -> Lxwp1 {
    Lxwp1 {
        product: None,
        serial: None,
        software_version: None,
        hardware_version: None,
        license: None,
    }
}

fn nonempty_text(value: &str) -> Option<Box<str>> {
    (!value.is_empty()).then(|| value.into())
}

fn omit_invalid_igc_identification_field(
    identification: &mut Lxwp1,
    field: &'static str,
    line_number: usize,
    warnings: &mut Vec<String>,
) {
    if Vec::<u8>::try_from(&*identification).is_ok() {
        return;
    }

    match field {
        "A" => identification.serial = None,
        "FTY" => identification.product = None,
        "RFW" => identification.software_version = None,
        "RHW" => identification.hardware_version = None,
        _ => unreachable!("mapped IGC identification field"),
    }
    warnings.push(format!("IGC line {line_number}: invalid {field} field"));
}

fn extension_value<T: FromStr>(
    record: &impl Extendable,
    extensions: &[Extension<'_>],
    mnemonic: &str,
    line_number: usize,
    warnings: &mut Vec<String>,
) -> Option<T> {
    let extension = extensions
        .iter()
        .find(|extension| extension.mnemonic == mnemonic)?;
    match record
        .get_extension(extension)
        .ok()
        .and_then(|value| value.parse().ok())
    {
        Some(value) => Some(value),
        None => {
            warnings.push(format!(
                "IGC line {line_number}: invalid {mnemonic} extension"
            ));
            None
        }
    }
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
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_ok, assert_some, assert_some_eq};
    use std::assert_matches;

    const BASIC: &[u8] = include_bytes!("../../../testdata/nmea/basic.nmea");
    const BASIC_IGC: &str = include_str!("../../../testdata/igc/basic.igc");
    const REPRESENTATIVE_IGC: &str = include_str!("../../../testdata/weglide_1141558.igc");

    #[test]
    fn loads_the_representative_igc_recording() {
        let replay = assert_ok!(Replay::from_igc(REPRESENTATIVE_IGC));
        let events = replay.events();
        let event_times = events.iter().map(ReplayEvent::at).collect::<Vec<_>>();
        let mut ordered_times = event_times.clone();
        ordered_times.sort_unstable();
        ordered_times.dedup();

        assert_eq!(event_times, ordered_times);
        assert_some_eq!(event_times.first().copied(), Duration::ZERO);
        assert_some_eq!(event_times.last().copied(), replay.duration());
        assert_eq!(replay.duration(), Duration::from_secs(18_817));
        assert!(replay.warnings().is_empty());

        for index in [0, events.len() / 2, events.len() - 1] {
            assert_payload_parses(&events[index]);
        }
        insta::assert_snapshot!(payload_text(&events[0]));
    }

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
    fn maps_igc_siu_extension_to_gga() {
        let replay = assert_ok!(Replay::from_igc(
            "I023638FXA3940SIU\nB1347495200000N00700000EA030480100012309\n"
        ));

        assert_some_eq!(first_gga(&replay.events()[0]).satellites_used, 9);
    }

    #[test]
    fn maps_igc_gsp_and_trt_extensions_to_rmc() {
        let replay = assert_ok!(Replay::from_igc(
            "I023640GSP4143TRT\nB1347495200000N00700000EA030480100018520271\n"
        ));
        let rmc = first_rmc(&replay.events()[0]);

        let speed = assert_some!(rmc.speed_over_ground);
        assert_abs_diff_eq!(speed.as_knots(), 100.0, epsilon = 1e-12);
        assert_some_eq!(
            rmc.course_over_ground.map(|track| track.as_degrees()),
            271.0
        );
    }

    #[test]
    fn maps_igc_tas_and_vat_extensions_to_lxwp0() {
        let replay = assert_ok!(Replay::from_igc(
            "I023640TAS4145VAT\nB1347495200000N00700000EA03048010003600000123\n"
        ));
        let lxwp0 = first_lxwp0(&replay.events()[0]);

        assert_some_eq!(
            lxwp0
                .true_airspeed
                .map(|speed| speed.as_kilometers_per_hour()),
            360.0
        );
        assert_some_eq!(lxwp0.pressure_altitude, Length::from_meters(3048.0));
        assert_eq!(lxwp0.vario_samples, [Speed::from_meters_per_second(1.23)]);
    }

    #[test]
    fn maps_igc_oat_extension_to_plxvs() {
        let replay = assert_ok!(Replay::from_igc(
            "I013639OAT\nB1347495200000N00700000EA0304801000-123\n"
        ));
        let plxvs = first_plxvs(&replay.events()[0]);

        assert_some_eq!(plxvs.outside_air_temperature, -12.3);
        assert_some_eq!(plxvs.igc_pressure_altitude, Length::from_meters(3048.0));
    }

    #[test]
    fn leaves_zero_pressure_altitude_empty_in_extension_sentences() {
        let replay = assert_ok!(Replay::from_igc(
            "I023640TAS4144OAT\nB1347495200000N00700000EA000000100036000-123\n"
        ));
        let event = &replay.events()[0];

        assert_none!(first_lxwp0(event).pressure_altitude);
        assert_none!(first_plxvs(event).igc_pressure_altitude);
    }

    #[test]
    fn ignores_igc_extension_bytes_without_an_i_definition() {
        let replay = assert_ok!(Replay::from_igc(
            "B1347495200000N00700000EA030480100009185202713600000123-123\n"
        ));
        let event = &replay.events()[0];

        let rmc = first_rmc(event);
        assert_none!(rmc.speed_over_ground);
        assert_none!(rmc.course_over_ground);
        assert_none!(first_gga(event).satellites_used);
        assert_eq!(payload_text(event).lines().count(), 4);
    }

    #[test]
    fn emits_all_i_defined_flight_extensions_in_sentence_order() {
        let replay = assert_ok!(Replay::from_igc(
            "HFDTE040726\n\
             I063637SIU3842GSP4345TRT4650TAS5155VAT5659OAT\n\
             B1347495200000N00700000EA030480100009185202713600000123-123\n"
        ));

        insta::assert_snapshot!(payload_text(&replay.events()[0]));
    }

    #[test]
    fn maps_igc_recorder_identification_to_lxwp1() {
        let replay = assert_ok!(Replay::from_igc(
            "ALXV123\n\
             HFFTYFRTYPE:LXNAV,LX9070PF\n\
             HFRFWFIRMWAREVERSION:9.54\n\
             HFRHWHARDWAREVERSION:38\n\
             B1347495200000N00700000EA0304801000\n"
        ));

        let identification = first_lxwp1(&replay.events()[0]);
        assert_some_eq!(identification.product, "LX9070PF".into());
        assert_some_eq!(identification.serial, "123".into());
        assert_some_eq!(identification.software_version, "9.54".into());
        assert_some_eq!(identification.hardware_version, "38".into());
        assert_none!(identification.license);
    }

    #[test]
    fn emits_lxwp1_with_empty_recorder_identification() {
        let replay = assert_ok!(Replay::from_igc("B1347495200000N00700000EA0304801000\n"));

        let identification = first_lxwp1(&replay.events()[0]);
        assert_none!(identification.product);
        assert_none!(identification.serial);
        assert_none!(identification.software_version);
        assert_none!(identification.hardware_version);
        assert_none!(identification.license);
    }

    #[test]
    fn repeats_lxwp1_every_minute_through_the_replay_duration() {
        let replay = assert_ok!(Replay::from_igc(
            "B1347495200000N00700000EA0304801000\n\
             B1349495200000N00700000EA0304801000\n"
        ));

        assert_eq!(
            replay
                .events()
                .iter()
                .map(ReplayEvent::at)
                .collect::<Vec<_>>(),
            [
                Duration::ZERO,
                Duration::from_secs(60),
                Duration::from_secs(120),
            ]
        );
        for event in replay.events() {
            first_lxwp1(event);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn restarts_lxwp1_cadence_at_time_zero_when_replay_loops() {
        let replay = assert_ok!(Replay::from_igc(
            "B1347495200000N00700000EA0304801000\n\
             B1347505200000N00700000EA0304801000\n"
        ));
        let (sender, mut receiver) = broadcast::channel(4);
        let playback = tokio::spawn(async move { replay.play(sender, Duration::ZERO, true).await });

        let first = assert_ok!(receiver.recv().await);
        parse_first_lxwp1(&first);

        tokio::time::advance(Duration::from_secs(1)).await;
        let last = assert_ok!(receiver.recv().await);
        let mut input = last.as_ref();
        assert_matches!(parse(&mut input), Step::Frame(Message::Rmc(_)));

        let restarted = assert_ok!(receiver.recv().await);
        parse_first_lxwp1(&restarted);
        playback.abort();
    }

    #[test]
    fn maps_j_defined_wind_extensions_from_an_unmatched_k_record() {
        let replay = assert_ok!(Replay::from_igc(
            "J020810WDI1115WSP\n\
             K13474927118520\n\
             B1347505200000N00700000EA0304801000\n"
        ));

        assert_eq!(
            replay
                .events()
                .iter()
                .map(ReplayEvent::at)
                .collect::<Vec<_>>(),
            [Duration::ZERO, Duration::from_secs(1)]
        );
        let wind = first_lxwp0(&replay.events()[0]);
        assert_some_eq!(
            wind.wind_direction.map(|direction| direction.as_degrees()),
            271.0
        );
        assert_some_eq!(
            wind.wind_speed.map(|speed| speed.as_kilometers_per_hour()),
            185.2
        );
    }

    #[test]
    fn merges_equal_time_b_and_k_values_into_one_lxwp0_sentence() {
        let replay = assert_ok!(Replay::from_igc(
            "I013640TAS\n\
             J020810WDI1115WSP\n\
             B1347495200000N00700000EA030480100036000\n\
             K13474927118520\n"
        ));
        let event = &replay.events()[0];

        assert_eq!(
            payload_text(event)
                .lines()
                .filter(|sentence| sentence.starts_with("$LXWP0"))
                .count(),
            1
        );
        let flight_data = first_lxwp0(event);
        assert_some_eq!(
            flight_data
                .true_airspeed
                .map(|speed| speed.as_kilometers_per_hour()),
            360.0
        );
        assert_some_eq!(
            flight_data
                .pressure_altitude
                .map(|altitude| altitude.as_meters()),
            3048.0
        );
        assert_some_eq!(
            flight_data
                .wind_direction
                .map(|direction| direction.as_degrees()),
            271.0
        );
        assert_some_eq!(
            flight_data
                .wind_speed
                .map(|speed| speed.as_kilometers_per_hour()),
            185.2
        );
    }

    #[test]
    fn does_not_carry_igc_wind_into_a_later_fix() {
        let replay = assert_ok!(Replay::from_igc(
            "I013640TAS\n\
             J020810WDI1115WSP\n\
             B1347495200000N00700000EA030480100036000\n\
             K13474927118520\n\
             B1347505200000N00700000EA030480100036000\n"
        ));

        let first = first_lxwp0(&replay.events()[0]);
        assert_some!(first.wind_direction);
        assert_some!(first.wind_speed);
        let second = first_lxwp0(&replay.events()[1]);
        assert_none!(second.wind_direction);
        assert_none!(second.wind_speed);
    }

    #[test]
    fn does_not_merge_wind_from_a_different_clamped_timestamp() {
        let replay = assert_ok!(Replay::from_igc(
            "I013640TAS\n\
             J020810WDI1115WSP\n\
             B1347505200000N00700000EA030480100036000\n\
             K13474927118520\n"
        ));

        assert_eq!(replay.events().len(), 1);
        assert_eq!(replay.warnings(), ["IGC line 4: timestamp moved backward"]);
        let flight_data = lxwp0_messages(&replay.events()[0]);
        assert_eq!(flight_data.len(), 2);
        assert_none!(flight_data[0].wind_direction);
        assert_none!(flight_data[0].wind_speed);
        assert_some!(flight_data[1].wind_direction);
        assert_some!(flight_data[1].wind_speed);
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
        assert_eq!(replay.warnings(), ["IGC line 2: timestamp moved backward"]);
    }

    #[test]
    fn rejects_igc_without_a_usable_b_record() {
        let error = Replay::from_igc("HFDTE040726\nB2400005200000N00700000EA0304801000\n");

        assert_matches!(error, Err(ReplayError::MissingFix));
    }

    #[test]
    fn warns_and_continues_after_a_malformed_mapped_record() {
        let replay = assert_ok!(Replay::from_igc(
            "Binvalid\n\
             B1347495200000N00700000EA0304801000\n"
        ));

        assert_eq!(replay.warnings(), ["IGC line 1: invalid B record"]);
        first_rmc(&replay.events()[0]);
    }

    #[test]
    fn ignores_unsupported_igc_records_without_warnings() {
        let replay = assert_ok!(Replay::from_igc(
            "Zunsupported\n\
             B1347495200000N00700000EA0304801000\n"
        ));

        assert!(replay.warnings().is_empty());
    }

    #[test]
    fn warns_and_omits_a_malformed_mapped_extension() {
        let replay = assert_ok!(Replay::from_igc(
            "I013640GSP\n\
             B1347495200000N00700000EA0304801000abcde\n"
        ));

        assert_eq!(replay.warnings(), ["IGC line 2: invalid GSP extension"]);
        assert_none!(first_rmc(&replay.events()[0]).speed_over_ground);
    }

    #[test]
    fn warns_about_a_malformed_date_and_preserves_rmc() {
        let replay = assert_ok!(Replay::from_igc(
            "HFDTEinvalid\n\
             B1347495200000N00700000EA0304801000\n"
        ));

        assert_eq!(replay.warnings(), ["IGC line 1: invalid DTE field"]);
        assert_none!(first_rmc(&replay.events()[0]).date);
    }

    #[test]
    fn warns_and_omits_only_the_invalid_recorder_field() {
        let replay = assert_ok!(Replay::from_igc(
            "HFFTYFRTYPE:LXNAV,LX9070PF\n\
             HFRFWFIRMWAREVERSION:9*54\n\
             B1347495200000N00700000EA0304801000\n"
        ));

        assert_eq!(replay.warnings(), ["IGC line 2: invalid RFW field"]);
        let identification = first_lxwp1(&replay.events()[0]);
        assert_some_eq!(identification.product, "LX9070PF".into());
        assert_none!(identification.software_version);
    }

    #[test]
    fn a_malformed_k_record_does_not_remove_its_matching_b_record() {
        let replay = assert_ok!(Replay::from_igc(
            "J020810WDI1115WSP\n\
             B1347495200000N00700000EA0304801000\n\
             Kinvalid\n"
        ));

        assert_eq!(replay.warnings(), ["IGC line 3: invalid K record"]);
        first_rmc(&replay.events()[0]);
        assert!(lxwp0_messages(&replay.events()[0]).is_empty());
    }

    #[test]
    fn skips_only_a_generated_sentence_that_cannot_be_encoded() {
        let replay = assert_ok!(Replay::from_igc(
            "HFDTE311299\n\
             B2359595200000N00700000EA0304801000\n\
             B0000005200000N00700000EA0304801000\n"
        ));

        assert_eq!(
            replay.warnings(),
            ["IGC line 3: RMC sentence cannot be encoded"]
        );
        assert!(rmc_messages(&replay.events()[1]).is_empty());
        first_gga(&replay.events()[1]);
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
        assert!(replay.warnings().is_empty());

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

    fn assert_payload_parses(event: &ReplayEvent) {
        let mut input = event.payload().as_ref();
        loop {
            match parse(&mut input) {
                Step::Frame(_) => {}
                Step::Incomplete => return,
                step => panic!("expected NMEA frame, got {step:?}"),
            }
        }
    }

    fn first_rmc(event: &ReplayEvent) -> Rmc {
        rmc_messages(event)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("expected RMC frame"))
    }

    fn rmc_messages(event: &ReplayEvent) -> Vec<Rmc> {
        let mut input = event.payload().as_ref();
        let mut messages = Vec::new();
        loop {
            match parse(&mut input) {
                Step::Frame(Message::Rmc(rmc)) => messages.push(rmc),
                Step::Frame(_) => {}
                Step::Incomplete => return messages,
                step => panic!("expected NMEA frame, got {step:?}"),
            }
        }
    }

    fn first_gga(event: &ReplayEvent) -> Gga {
        let mut input = event.payload().as_ref();
        loop {
            match parse(&mut input) {
                Step::Frame(Message::Gga(gga)) => return gga,
                Step::Frame(_) => {}
                step => panic!("expected GGA frame, got {step:?}"),
            }
        }
    }

    fn first_lxwp0(event: &ReplayEvent) -> Lxwp0 {
        lxwp0_messages(event)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("expected LXWP0 frame"))
    }

    fn first_lxwp1(event: &ReplayEvent) -> Lxwp1 {
        parse_first_lxwp1(event.payload())
    }

    fn parse_first_lxwp1(payload: &[u8]) -> Lxwp1 {
        let mut input = payload;
        match parse(&mut input) {
            Step::Frame(Message::Lxwp1(lxwp1)) => lxwp1,
            step => panic!("expected LXWP1 frame, got {step:?}"),
        }
    }

    fn lxwp0_messages(event: &ReplayEvent) -> Vec<Lxwp0> {
        let mut input = event.payload().as_ref();
        let mut messages = Vec::new();
        loop {
            match parse(&mut input) {
                Step::Frame(Message::Lxwp0(lxwp0)) => messages.push(lxwp0),
                Step::Frame(_) => {}
                Step::Incomplete => return messages,
                step => panic!("expected NMEA frame, got {step:?}"),
            }
        }
    }

    fn first_plxvs(event: &ReplayEvent) -> updraft_nmea::Plxvs {
        let mut input = event.payload().as_ref();
        loop {
            match parse(&mut input) {
                Step::Frame(Message::Plxvs(plxvs)) => return plxvs,
                Step::Frame(_) => {}
                step => panic!("expected PLXVS frame, got {step:?}"),
            }
        }
    }
}
