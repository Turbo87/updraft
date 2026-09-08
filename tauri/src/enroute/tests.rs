use super::*;
use claims::{assert_err, assert_ok};
use serde_json::json;

#[test]
fn parses_supported_regions_in_path_order() {
    let catalog = json!({"maps": [
        {"path":"North America/USA West.mbtiles", "size":200, "time":"20260801"},
        {"path":"Europe/Germany.mbtiles", "size":47251456, "time":"20251112", "bbox":[]},
        {"path":"Europe/Northern Ireland.mbtiles", "size":400, "time":"20260228"},
        {"path":"Europe/Great Britain.mbtiles", "size":300, "time":"20240229"},
        {"path":"Africa/Canary Islands.mbtiles", "size":100, "time":"20260908"},
        {"path":"Africa/Reunion.mbtiles", "size":500, "time":"20260801"},
        {"path":"Asia/Japan.mbtiles", "size":600, "time":"20260801"},
        {"path":"Australia Oceanica/Australia.mbtiles", "size":700, "time":"20260801"},
        {"path":"South America/Falkland Islands.mbtiles", "size":800, "time":"20260801"},
        {"path":"Europe/France.geojson"},
        {"path":"Europe/Future Country.mbtiles", "size":-1, "time":"invalid"}
    ], "minAppVersion":"999", "url":"https://example.invalid"});
    let files = assert_ok!(parse_catalog(&serde_json::to_vec(&catalog).unwrap()));
    insta::assert_debug_snapshot!(files);
}

#[test]
fn rejects_malformed_catalogs_and_supported_entries() {
    for catalog in [
        json!(null),
        json!({}),
        json!({"maps":{}}),
        json!({"maps":[{}]}),
    ] {
        assert_err!(parse_catalog(&serde_json::to_vec(&catalog).unwrap()));
    }
    assert_err!(parse_catalog(b"not JSON"));
    for date in [
        "20260229",
        "20261301",
        "20260001",
        "20260100",
        "2026-01-01",
        "2026011",
        "abcdefgh",
    ] {
        let catalog = json!({"maps":[{"path":"Europe/Germany.mbtiles", "size":1, "time":date}]});
        assert_err!(parse_catalog(&serde_json::to_vec(&catalog).unwrap()));
    }
    for size in [json!(0), json!(-1), json!(1.5), json!("10"), json!(null)] {
        let catalog =
            json!({"maps":[{"path":"Europe/Germany.mbtiles", "size":size, "time":"20260908"}]});
        assert_err!(parse_catalog(&serde_json::to_vec(&catalog).unwrap()));
    }
    let entry = json!({"path":"Europe/Germany.mbtiles", "size":1, "time":"20260908"});
    let duplicate = json!({"maps":[entry, entry]});
    assert_err!(parse_catalog(&serde_json::to_vec(&duplicate).unwrap()));
}

#[test]
fn omits_paths_outside_the_bundled_mapping() {
    let paths = [
        "../Europe/Germany.mbtiles",
        "/Europe/Germany.mbtiles",
        "Europe/../Germany.mbtiles",
        "Europe\\Germany.mbtiles",
        "Europe/%47ermany.mbtiles",
        "Europe/Germany.mbtiles?x=1",
        "https://example.com/Europe/Germany.mbtiles",
        "Europe/Germany.mbtiles/extra",
    ];
    let maps: Vec<_> = paths.into_iter().map(|path| json!({"path":path})).collect();
    let bytes = serde_json::to_vec(&json!({"maps":maps})).unwrap();
    assert!(assert_ok!(parse_catalog(&bytes)).is_empty());
    assert!(assert_ok!(parse_catalog(br#"{"maps":[]}"#)).is_empty());
}

#[test]
fn bundled_regions_have_unique_paths_and_consistent_country_groups() {
    let mut paths = std::collections::BTreeSet::new();
    let mut countries = BTreeMap::new();
    for &(path, country, continent) in regions::REGIONS {
        assert!(paths.insert(path), "Duplicate region: {path}");
        if let Some(previous) = countries.insert(country, continent) {
            assert_eq!(previous, continent, "Inconsistent continent for {country}");
        }
    }
}

#[test]
fn parses_terrain_and_basemap_entries_for_the_same_region() {
    let catalog = json!({"maps":[
        {"path":"Europe/France.mbtiles","size":20,"time":"20260908"},
        {"path":"Europe/France.terrain","size":10,"time":"20260909"}
    ]});
    let files = assert_ok!(parse_catalog(&serde_json::to_vec(&catalog).unwrap()));
    assert_eq!(files.len(), 2);
    assert_eq!(files[1].path, "Europe/France.terrain");
    assert_eq!(files[1].country_code, "FR");
    assert_eq!(files[1].continent, Continent::Europe);
    assert_eq!(files[1].size.get(), 10);
    assert_eq!(
        files[1].publication_date,
        time::macros::date!(2026 - 09 - 09)
    );
    let entry = &catalog["maps"][1];
    let duplicate = json!({"maps":[entry,entry]});
    assert_err!(parse_catalog(&serde_json::to_vec(&duplicate).unwrap()));
    let invalid = br#"{"maps":[{"path":"Europe/France.terrain","size":0,"time":"20260909"}]}"#;
    assert_err!(parse_catalog(invalid));
}
