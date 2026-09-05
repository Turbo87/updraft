use claims::assert_ok;
use std::{collections::BTreeMap, sync::Arc};
use updraft_airspace::AirspaceDataset;
use updraft_core::{AirspaceCatalog, AirspaceLoadError, AirspaceSourceStatus};

#[test]
fn catalog_keeps_duplicate_airspaces_and_unavailable_sources_independent() {
    let bytes = include_bytes!("../../../testdata/airspace/polygon.txt");
    let dataset = Arc::new(assert_ok!(AirspaceDataset::from_openair(bytes)));
    let catalog = AirspaceCatalog {
        sources: BTreeMap::from([
            ("a.txt".into(), Ok(dataset.clone())),
            ("b.txt".into(), Ok(dataset)),
            ("broken.txt".into(), Err(AirspaceLoadError::ParseFailed)),
        ]),
    };
    assert_eq!(
        catalog.source_statuses(),
        vec![
            AirspaceSourceStatus::Active {
                source_name: "a.txt".into(),
                airspace_count: 1
            },
            AirspaceSourceStatus::Active {
                source_name: "b.txt".into(),
                airspace_count: 1
            },
            AirspaceSourceStatus::Unavailable {
                source_name: "broken.txt".into(),
                error: AirspaceLoadError::ParseFailed
            },
        ]
    );
}
