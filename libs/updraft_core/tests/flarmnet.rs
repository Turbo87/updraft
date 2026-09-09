use claims::{assert_err, assert_none, assert_ok, assert_some};
use updraft_core::{FlarmnetDatabase, TrafficTargetId, TrafficTargetIdType};

#[test]
fn looks_up_flarm_and_icao_ids_without_matching_random_ids() {
    let json = br#"[{"flarm_id":"aBc123","call_sign":" EL ","registration":"D-TEST",
        "plane_type":"AS 33","pilot_name":"Example Pilot","airfield":"Example",
        "frequency":"123.450"}]"#;
    let database = assert_ok!(FlarmnetDatabase::from_json(json));
    let flarm = TrafficTargetId::new(TrafficTargetIdType::Flarm, 0xABC123);
    let icao = TrafficTargetId::new(TrafficTargetIdType::Icao, 0xABC123);
    let record = assert_some!(database.lookup(flarm));
    claims::assert_some_eq!(database.lookup(icao), record);
    insta::assert_json_snapshot!(record);
    for id_type in [TrafficTargetIdType::Random, TrafficTargetIdType::Other(4)] {
        assert_none!(database.lookup(TrafficTargetId::new(id_type, 0xABC123)));
    }
    assert_none!(database.lookup(TrafficTargetId::new(TrafficTargetIdType::Flarm, 1)));
}

#[test]
fn accepts_missing_fields_and_ignores_unknown_fields() {
    let json = br#"[{"flarm_id":"000001","call_sign":"  ","future_field":true}]"#;
    let database = assert_ok!(FlarmnetDatabase::from_json(json));
    let id = TrafficTargetId::new(TrafficTargetIdType::Flarm, 1);
    insta::assert_debug_snapshot!(assert_some!(database.lookup(id)));
}

#[test]
fn rejects_invalid_databases() {
    for json in [
        "not JSON",
        "[]",
        r#"[{"flarm_id":"12345"}]"#,
        r#"[{"flarm_id":"1000000"}]"#,
        r#"[{"flarm_id":"GG0000"}]"#,
        r#"[{"flarm_id":"+00001"}]"#,
        r#"[{"call_sign":"EL"}]"#,
        r#"[{"flarm_id":"abcdef"},{"flarm_id":"ABCDEF"}]"#,
    ] {
        assert_err!(FlarmnetDatabase::from_json(json.as_bytes()), "{json}");
    }
}
