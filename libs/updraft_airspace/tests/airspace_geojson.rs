use claims::{assert_none, assert_ok, assert_some};
use serde_json::json;
use time::{Date, Month, Time, UtcOffset, Weekday};
use updraft_airspace::{
    Airspace, AirspaceActivity, AirspaceAltitude, AirspaceDataset, AirspaceId,
    AirspaceOperatingHours, AirspaceOperatingPeriod, AirspaceOperatingSchedule,
};
use updraft_units::{Length, PressureAltitude};

const POLYGON: &[u8] = include_bytes!("../../../testdata/airspace/polygon.txt");
const ALTITUDES: &[u8] = include_bytes!("../../../testdata/airspace/altitudes.txt");
const COMPLETE_AIRSPACE: &[u8] = b"AC D
AY R
AN Complete
AF 123.45
AG TOWER
AX 123
AL GND
AH 5000 FT
DP 50:00:00 N 010:00:00 E
DP 50:00:00 N 010:01:00 E
DP 50:01:00 N 010:01:00 E
DP 50:01:00 N 010:00:00 E
";

fn complete_airspace() -> Airspace {
    let dataset = assert_ok!(AirspaceDataset::from_openair(COMPLETE_AIRSPACE));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.activity = Some(AirspaceActivity::HangGlidingOrParagliding);
    airspace.on_demand = Some(true);
    airspace.on_request = Some(false);
    airspace.by_notam = Some(true);
    airspace.special_agreement = Some(false);
    airspace.request_compliance = Some(true);
    airspace.country_codes = vec!["DE".into(), "AT".into()];
    airspace.lower_limit_min = Some(AirspaceAltitude::Agl(Length::from_feet(500.)));
    airspace.upper_limit_max = Some(AirspaceAltitude::FlightLevel(PressureAltitude::new(
        Length::from_feet(12_000.),
    )));
    airspace.frequencies[0].remarks = Some("EMERGENCIES ONLY".into());
    airspace.transponder_settings[0].remarks = Some("WHEN ACTIVE".into());
    airspace.hours_of_operation = Some(assert_some!(AirspaceOperatingHours::new(
        vec![AirspaceOperatingPeriod {
            day_of_week: Weekday::Sunday,
            public_holidays_excluded: true,
            remarks: Some("DAYLIGHT HOURS".into()),
            schedule: AirspaceOperatingSchedule::SunriseUntilSunset,
        }],
        Some("LOCAL TIME".into()),
    )));
    let date = assert_ok!(Date::from_calendar_date(2026, Month::April, 12));
    airspace.active_from = Some(assert_ok!(date.with_hms(8, 30, 0)).assume_utc());
    airspace.active_until = Some(assert_ok!(date.with_hms(17, 45, 0)).assume_utc());
    airspace.remarks = Some("ACTIVE DURING GLIDER EVENTS".into());
    airspace
}

#[test]
fn projects_complete_airspace_as_geojson() {
    insta::assert_json_snapshot!(complete_airspace().to_geojson(AirspaceId(0)));
}

#[test]
fn projects_required_openaip_classification_properties() {
    let legacy_class = b"AC R\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:01:00 N 010:00:00 E\n";
    let dataset = assert_ok!(AirspaceDataset::from_openair(legacy_class));

    let properties = &dataset.airspaces()[0].to_geojson(AirspaceId(0))["properties"];
    assert_eq!(properties["icaoClass"], json!(8));
    assert_eq!(properties["type"], json!(1));
    assert_none!(properties.get("activity"));
}

#[test]
fn projects_optional_openaip_activity() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.activity = Some(AirspaceActivity::HangGlidingOrParagliding);

    assert_eq!(
        airspace.to_geojson(AirspaceId(0))["properties"]["activity"],
        json!(5)
    );
}

#[test]
fn projects_optional_airspace_name() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));

    assert_eq!(
        dataset.airspaces()[0].to_geojson(AirspaceId(0))["properties"]["name"],
        json!("Polygon")
    );
}

#[test]
fn omits_absent_airspace_name() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.name = None;

    assert_none!(airspace.to_geojson(AirspaceId(0))["properties"].get("name"));
}

#[test]
fn projects_openaip_vertical_limits() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(ALTITUDES));
    let properties = dataset
        .airspaces()
        .iter()
        .map(|airspace| airspace.to_geojson(AirspaceId(0))["properties"].clone())
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(properties);
}

#[test]
fn projects_optional_openaip_vertical_constraints() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.lower_limit_min = Some(AirspaceAltitude::Agl(Length::from_feet(500.)));
    airspace.upper_limit_max = Some(AirspaceAltitude::FlightLevel(PressureAltitude::new(
        Length::from_feet(12_000.),
    )));

    insta::assert_json_snapshot!(airspace.to_geojson(AirspaceId(0))["properties"]);
}

#[test]
fn projects_openaip_operational_flags() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.on_demand = Some(true);
    airspace.on_request = Some(false);
    airspace.by_notam = Some(true);
    airspace.special_agreement = Some(false);
    airspace.request_compliance = Some(true);

    let properties = &airspace.to_geojson(AirspaceId(0))["properties"];
    assert_eq!(properties["onDemand"], json!(true));
    assert_eq!(properties["onRequest"], json!(false));
    assert_eq!(properties["byNotam"], json!(true));
    assert_eq!(properties["specialAgreement"], json!(false));
    assert_eq!(properties["requestCompliance"], json!(true));
}

#[test]
fn omits_absent_openaip_operational_flags() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let properties = &dataset.airspaces()[0].to_geojson(AirspaceId(0))["properties"];

    for property in [
        "onDemand",
        "onRequest",
        "byNotam",
        "specialAgreement",
        "requestCompliance",
    ] {
        assert_none!(properties.get(property));
    }
}

#[test]
fn projects_one_country_as_a_scalar() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.country_codes = vec!["DE".into()];

    assert_eq!(
        airspace.to_geojson(AirspaceId(0))["properties"]["country"],
        json!("DE")
    );
}

#[test]
fn projects_multiple_countries_as_an_ordered_array() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.country_codes = vec!["DE".into(), "AT".into()];

    assert_eq!(
        airspace.to_geojson(AirspaceId(0))["properties"]["country"],
        json!(["DE", "AT"])
    );
}

#[test]
fn omits_an_empty_country_collection() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));

    assert_none!(dataset.airspaces()[0].to_geojson(AirspaceId(0))["properties"].get("country"));
}

#[test]
fn preserves_an_unrecognized_country_value() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.country_codes = vec!["UNKNOWN".into()];

    assert_eq!(
        airspace.to_geojson(AirspaceId(0))["properties"]["country"],
        json!("UNKNOWN")
    );
}

#[test]
fn projects_openaip_frequencies() {
    let bytes = b"AC D\nAF 123.45\nAG TOWER\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:01:00 N 010:00:00 E\n";
    let dataset = assert_ok!(AirspaceDataset::from_openair(bytes));
    let mut airspace = dataset.airspaces()[0].clone();
    let mut secondary = airspace.frequencies[0].clone();
    secondary.name = None;
    secondary.primary = None;
    secondary.remarks = Some("GUARD".into());
    airspace.frequencies.push(secondary);

    assert_eq!(
        airspace.to_geojson(AirspaceId(0))["properties"]["frequencies"],
        json!([
            {
                "value": "123.450",
                "unit": 2,
                "name": "TOWER",
                "primary": true,
            },
            {
                "value": "123.450",
                "unit": 2,
                "remarks": "GUARD",
            },
        ])
    );
}

#[test]
fn projects_openaip_transponder_settings() {
    let bytes = b"AC D\nAX 123\nAL GND\nAH FL100\nDP 50:00:00 N 010:00:00 E\nDP 50:00:00 N 010:01:00 E\nDP 50:01:00 N 010:00:00 E\n";
    let dataset = assert_ok!(AirspaceDataset::from_openair(bytes));
    let mut airspace = dataset.airspaces()[0].clone();
    let mut secondary = airspace.transponder_settings[0].clone();
    secondary.primary = false;
    secondary.remarks = Some("WHEN ACTIVE".into());
    airspace.transponder_settings.push(secondary);

    assert_eq!(
        airspace.to_geojson(AirspaceId(0))["properties"]["transponderSettings"],
        json!([
            {
                "code": "0123",
                "primary": true,
            },
            {
                "code": "0123",
                "primary": false,
                "remarks": "WHEN ACTIVE",
            },
        ])
    );
}

#[test]
fn projects_openaip_operating_schedules() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    let period = |day_of_week, schedule| AirspaceOperatingPeriod {
        day_of_week,
        public_holidays_excluded: false,
        remarks: None,
        schedule,
    };
    let periods = vec![
        AirspaceOperatingPeriod {
            day_of_week: Weekday::Monday,
            public_holidays_excluded: true,
            remarks: Some("WEEKDAYS".into()),
            schedule: AirspaceOperatingSchedule::Fixed {
                start_time: assert_ok!(Time::from_hms(8, 30, 15)),
                end_time: assert_ok!(Time::from_hms(17, 45, 0)),
            },
        },
        period(
            Weekday::Tuesday,
            AirspaceOperatingSchedule::FixedStartUntilSunset {
                start_time: assert_ok!(Time::from_hms(9, 0, 0)),
            },
        ),
        period(
            Weekday::Wednesday,
            AirspaceOperatingSchedule::SunriseUntilFixedEnd {
                end_time: assert_ok!(Time::from_hms(18, 0, 0)),
            },
        ),
        period(
            Weekday::Thursday,
            AirspaceOperatingSchedule::SunriseUntilSunset,
        ),
        period(Weekday::Friday, AirspaceOperatingSchedule::NoSpecifiedTime),
        period(Weekday::Saturday, AirspaceOperatingSchedule::ByNotam),
    ];
    airspace.hours_of_operation = Some(assert_some!(AirspaceOperatingHours::new(
        periods,
        Some("LOCAL TIME".into()),
    )));

    insta::assert_json_snapshot!(
        airspace.to_geojson(AirspaceId(0))["properties"]["hoursOfOperation"]
    );
}

#[test]
fn projects_independently_optional_openaip_activation_dates() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    let date = assert_ok!(Date::from_calendar_date(2026, Month::April, 12));
    let offset = assert_ok!(UtcOffset::from_hms(2, 0, 0));
    let active_from = assert_ok!(date.with_hms(8, 30, 0)).assume_offset(offset);
    let active_until = assert_ok!(date.with_hms(9, 45, 30)).assume_utc();
    airspace.active_from = Some(active_from);
    airspace.active_until = Some(active_until);

    let properties = &airspace.to_geojson(AirspaceId(0))["properties"];
    assert_eq!(properties["activeFrom"], json!("2026-04-12T08:30:00+02:00"));
    assert_eq!(properties["activeUntil"], json!("2026-04-12T09:45:30Z"));

    airspace.active_from = None;
    let properties = &airspace.to_geojson(AirspaceId(0))["properties"];
    assert_none!(properties.get("activeFrom"));
    assert_eq!(properties["activeUntil"], json!("2026-04-12T09:45:30Z"));

    airspace.active_from = Some(active_from);
    airspace.active_until = None;
    let properties = &airspace.to_geojson(AirspaceId(0))["properties"];
    assert_eq!(properties["activeFrom"], json!("2026-04-12T08:30:00+02:00"));
    assert_none!(properties.get("activeUntil"));
}

#[test]
fn projects_optional_airspace_remarks() {
    let dataset = assert_ok!(AirspaceDataset::from_openair(POLYGON));
    let mut airspace = dataset.airspaces()[0].clone();
    airspace.remarks = Some("ACTIVE DURING GLIDER EVENTS".into());

    assert_eq!(
        airspace.to_geojson(AirspaceId(0))["properties"]["remarks"],
        json!("ACTIVE DURING GLIDER EVENTS")
    );

    airspace.remarks = None;
    assert_none!(airspace.to_geojson(AirspaceId(0))["properties"].get("remarks"));
}
