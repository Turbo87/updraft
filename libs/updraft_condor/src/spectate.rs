//! The `Spectate.json` player list, and its conversion into FLARM traffic
//! sentences.

use serde_json::Value;
use updraft_geo::LatLon;
use updraft_nmea::{
    FlarmAircraftType, FlarmAlarmLevel, FlarmId, FlarmIdType, Pflaa, Pflau, PflauAlarmType,
    PflauGpsStatus,
};
use updraft_units::{Angle, Length, Speed};

/// One player with a usable position.
#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub id: String,
    pub competition_number: Option<String>,
    pub registration: Option<String>,
    pub position: LatLon,
    pub altitude: Option<Length>,
    pub speed: Option<Speed>,
    pub heading: Option<Angle>,
    pub vario: Option<Speed>,
}

/// The own position and altitude that the relative positions refer to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reference {
    pub position: LatLon,
    pub altitude: Option<Length>,
}

/// Parses the file content. Condor writes one array of player objects. A
/// single object is accepted as well. Players without a valid position
/// are left out.
pub fn parse_snapshot(content: &[u8]) -> Result<Vec<Player>, serde_json::Error> {
    let content = content.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(content);
    let value: Value = serde_json::from_slice(content)?;
    let objects = match value {
        Value::Array(items) => items,
        object @ Value::Object(_) => vec![object],
        _ => Vec::new(),
    };
    Ok(objects.iter().filter_map(player).collect())
}

fn player(value: &Value) -> Option<Player> {
    let object = value.as_object()?;
    let id = match object.get("ID")? {
        Value::String(id) => id.clone(),
        Value::Number(id) => id.to_string(),
        _ => return None,
    };
    let latitude = coordinate(object.get("latitude")?, 90.0)?;
    let longitude = coordinate(object.get("longitude")?, 180.0)?;
    Some(Player {
        id,
        competition_number: text(object.get("CN")),
        registration: text(object.get("RN")),
        position: LatLon::from_degrees(latitude, longitude),
        altitude: object
            .get("altitude")
            .and_then(number)
            .map(Length::from_meters),
        speed: object
            .get("speed")
            .and_then(number)
            .map(Speed::from_kilometers_per_hour),
        heading: object
            .get("heading")
            .and_then(number)
            .map(Angle::from_degrees),
        vario: object
            .get("vario")
            .and_then(number)
            .map(Speed::from_meters_per_second),
    })
}

/// A Condor coordinate: a hemisphere letter followed by decimal degrees,
/// such as `N45.000000` or `W008.500000`.
fn coordinate(value: &Value, limit: f64) -> Option<f64> {
    let text = value.as_str()?.trim();
    let (hemisphere, digits) = text.split_at_checked(1)?;
    let sign = match hemisphere {
        "N" | "n" | "E" | "e" => 1.0,
        "S" | "s" | "W" | "w" => -1.0,
        _ => return None,
    };
    let degrees: f64 = digits.trim().parse().ok()?;
    (degrees.is_finite() && degrees <= limit).then_some(sign * degrees)
}

/// Condor writes numbers as strings. A JSON number is accepted as well.
fn number(value: &Value) -> Option<f64> {
    let number = match value {
        Value::Number(number) => number.as_f64()?,
        Value::String(text) => text.trim().parse().ok()?,
        _ => return None,
    };
    number.is_finite().then_some(number)
}

fn text(value: Option<&Value>) -> Option<String> {
    let text = value?.as_str()?.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// The own entry: the player with this competition number, or with this
/// registration when no competition number matches.
pub fn own_player<'a>(players: &'a [Player], competition_number: &str) -> Option<&'a Player> {
    let matches = |value: &Option<String>| {
        value
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case(competition_number))
    };
    players
        .iter()
        .find(|player| matches(&player.competition_number))
        .or_else(|| players.iter().find(|player| matches(&player.registration)))
}

/// One `$PFLAA` per other player and one closing `$PFLAU`.
pub fn traffic_sentences(
    players: &[Player],
    own: Option<&Player>,
    reference: Reference,
) -> Vec<u8> {
    let mut output = Vec::new();
    let mut count = 0u8;
    for player in players {
        if own.is_some_and(|own| own.id == player.id) {
            continue;
        }
        let (distance, bearing) = reference.position.distance_bearing(player.position);
        let (sin, cos) = bearing.sin_cos();
        // Adding zero turns a rounded `-0` into `0`.
        let north = (distance.as_meters() * cos).round() + 0.0;
        let east = (distance.as_meters() * sin).round() + 0.0;
        let relative_vertical = match (player.altitude, reference.altitude) {
            (Some(altitude), Some(own_altitude)) => Some(Length::from_meters(
                (altitude.as_meters() - own_altitude.as_meters()).round(),
            )),
            _ => None,
        };
        let pflaa = Pflaa {
            alarm_level: FlarmAlarmLevel::None,
            relative_north: Some(Length::from_meters(north)),
            relative_east: Some(Length::from_meters(east)),
            relative_vertical,
            id_type: Some(FlarmIdType::Flarm),
            id: Some(FlarmId {
                address: flarm_address(&player.id),
                callsign: player
                    .competition_number
                    .as_deref()
                    .filter(|number| number.is_ascii())
                    .map(Into::into),
            }),
            track: player
                .heading
                .map(|heading| Angle::from_degrees(heading.normalized().as_degrees().round())),
            turn_rate: None,
            ground_speed: player.speed.map(|speed| {
                Speed::from_meters_per_second(speed.as_meters_per_second().max(0.0).round())
            }),
            climb_rate: player.vario.map(|vario| {
                Speed::from_meters_per_second((vario.as_meters_per_second() * 10.0).round() / 10.0)
            }),
            aircraft_type: FlarmAircraftType::Glider,
            no_track: None,
            source: None,
            rssi: None,
        };
        if let Ok(sentence) = Vec::<u8>::try_from(&pflaa) {
            output.extend_from_slice(&sentence);
            count = count.saturating_add(1);
        }
    }

    let pflau = Pflau {
        rx_count: Some(count),
        tx_ok: Some(true),
        gps_status: PflauGpsStatus::Airborne,
        power_ok: Some(true),
        alarm_level: FlarmAlarmLevel::None,
        relative_bearing: None,
        alarm_type: PflauAlarmType::None,
        relative_vertical: None,
        relative_distance: None,
        id: None,
    };
    if let Ok(sentence) = Vec::<u8>::try_from(&pflau) {
        output.extend_from_slice(&sentence);
    }
    output
}

/// A stable 24-bit address from the Condor player ID: the low 24 bits of a
/// numeric ID, or an FNV-1a hash of any other text. Zero is avoided
/// because it reads as "no ID" on some consumers.
fn flarm_address(id: &str) -> u32 {
    let value = match id.trim().parse::<u64>() {
        Ok(number) => number,
        Err(_) => id.bytes().fold(0xCBF2_9CE4_8422_2325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3)
        }),
    };
    let address = (value & 0xFF_FFFF) as u32;
    if address == 0 { 1 } else { address }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_ok, assert_some, assert_some_eq};

    const SAMPLE: &[u8] = include_bytes!("../../../testdata/condor/spectate.json");

    /// The fixture from the `XCSoar` `Condor3Spectate` driver tests.
    const XCSOAR_FIXTURE: &str = r#"[
  {"ID":"1","CN":"AA","latitude":"N45.000000","longitude":"E013.000000","altitude":"1000","speed":"100","heading":"90","vario":"0"},
  {"ID":"2","CN":"BB","latitude":"N45.000000","longitude":"E013.000000","altitude":"1100","speed":"100","heading":"90","vario":"0"}
]"#;

    #[test]
    fn parses_players_with_a_position_and_skips_the_rest() {
        let players = assert_ok!(parse_snapshot(SAMPLE));
        assert_eq!(players.len(), 3);
        let jane = &players[1];
        assert_eq!(jane.id, "2074615532");
        assert_some_eq!(jane.competition_number.clone(), "XY".to_owned());
        assert_some_eq!(jane.registration.clone(), "D-KXYZ".to_owned());
        assert_eq!(jane.position, LatLon::from_degrees(45.9, 13.9));
        assert_some_eq!(jane.altitude, Length::from_meters(1250.0));
        assert_some_eq!(jane.speed, Speed::from_kilometers_per_hour(112.5));
        assert_some_eq!(jane.heading, Angle::from_degrees(95.0));
        assert_some_eq!(jane.vario, Speed::from_meters_per_second(-1.34));

        let tow = &players[2];
        assert_eq!(tow.position, LatLon::from_degrees(45.895, 13.89));
        assert_none!(tow.competition_number.as_deref());
        assert_none!(tow.altitude);
        assert_none!(tow.speed);
    }

    #[test]
    fn accepts_a_byte_order_mark_and_a_single_object() {
        let content = b"\xEF\xBB\xBF{\"ID\":5,\"latitude\":\"N45.5\",\"longitude\":\"E013\",\"altitude\":1200}";
        let players = assert_ok!(parse_snapshot(content));
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].id, "5");
        assert_some_eq!(players[0].altitude, Length::from_meters(1200.0));
    }

    #[test]
    fn rejects_a_truncated_file() {
        claims::assert_err!(parse_snapshot(&SAMPLE[..SAMPLE.len() / 2]));
    }

    #[test]
    fn finds_the_own_player_by_competition_number_then_registration() {
        let players = assert_ok!(parse_snapshot(SAMPLE));
        assert_eq!(assert_some!(own_player(&players, "xy")).id, "2074615532");
        assert_eq!(
            assert_some!(own_player(&players, "F-CTJD")).id,
            "3103807898"
        );
        assert_none!(own_player(&players, "ZZ"));
    }

    #[test]
    fn converts_the_xcsoar_fixture() {
        let players = assert_ok!(parse_snapshot(XCSOAR_FIXTURE.as_bytes()));
        let own = assert_some!(own_player(&players, "AA"));
        let reference = Reference {
            position: own.position,
            altitude: own.altitude,
        };
        let output = traffic_sentences(&players, Some(own), reference);
        insta::assert_snapshot!(String::from_utf8(output).unwrap());
    }

    #[test]
    fn converts_the_sample_snapshot() {
        let players = assert_ok!(parse_snapshot(SAMPLE));
        let own = assert_some!(own_player(&players, "XY"));
        let reference = Reference {
            position: own.position,
            altitude: own.altitude,
        };
        let output = traffic_sentences(&players, Some(own), reference);
        insta::assert_snapshot!(String::from_utf8(output).unwrap());
    }

    #[test]
    fn without_an_own_entry_every_player_is_traffic() {
        let players = assert_ok!(parse_snapshot(XCSOAR_FIXTURE.as_bytes()));
        let reference = Reference {
            position: LatLon::from_degrees(45.0, 13.0),
            altitude: None,
        };
        let output = String::from_utf8(traffic_sentences(&players, None, reference)).unwrap();
        assert_eq!(output.matches("$PFLAA").count(), 2);
        assert!(output.contains("$PFLAU,2,"));
        assert!(output.contains("$PFLAA,0,0,0,,2,000001!AA,"));
    }

    #[test]
    fn derives_stable_addresses() {
        assert_eq!(
            flarm_address("3103807898"),
            (3_103_807_898u64 & 0xFF_FFFF) as u32
        );
        assert_eq!(flarm_address("0"), 1);
        assert_eq!(flarm_address("abc"), flarm_address("abc"));
        assert_ne!(flarm_address("abc"), flarm_address("abd"));
    }
}
