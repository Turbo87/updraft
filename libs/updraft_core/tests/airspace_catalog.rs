use claims::assert_ok;
use std::{collections::BTreeMap, sync::Arc};
use updraft_airspace::AirspaceDataset;
use updraft_core::{AirspaceCatalog, AirspaceLoadError, AirspaceSource, AirspaceSourceStatus};

#[test]
fn catalog_keeps_duplicate_airspaces_and_source_states_independent() {
    let bytes = include_bytes!("../../../testdata/airspace/polygon.txt");
    let dataset = Arc::new(assert_ok!(AirspaceDataset::from_openair(bytes)));
    let catalog = AirspaceCatalog {
        sources: BTreeMap::from([
            ("a.txt".into(), AirspaceSource::Active(dataset.clone())),
            ("b.txt".into(), AirspaceSource::Active(dataset)),
            (
                "broken.txt".into(),
                AirspaceSource::Unavailable(AirspaceLoadError::ParseFailed),
            ),
            ("disabled.txt".into(), AirspaceSource::Disabled),
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
            AirspaceSourceStatus::Disabled {
                source_name: "disabled.txt".into()
            },
        ]
    );
}

#[test]
fn replacing_catalog_publishes_status_and_keeps_old_snapshots_immutable() {
    use updraft_core::{
        Core, GetAirspaceSnapshot, ReplaceAirspaceCatalog, SettingsSnapshot, Timestamp, Topic,
    };
    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::from_millis(0);
    let initial = core.apply(GetAirspaceSnapshot, at).response;
    let catalog = Arc::new(AirspaceCatalog {
        sources: BTreeMap::from([(
            "broken.txt".into(),
            AirspaceSource::Unavailable(AirspaceLoadError::ReadFailed),
        )]),
    });
    let update = core.apply(ReplaceAirspaceCatalog(catalog.clone()), at);
    let snapshot = core.apply(GetAirspaceSnapshot, at).response;
    assert!(Arc::ptr_eq(&snapshot.catalog, &catalog));
    assert_eq!(snapshot.generation, 1);
    assert_eq!(initial.generation, 0);
    assert_eq!(initial.catalog.sources.len(), 0);
    assert_eq!(
        update.effects,
        vec![updraft_core::Effect::emit(Topic::Airspace(
            catalog.status(1)
        ))]
    );
    core.apply(
        ReplaceAirspaceCatalog(Arc::new(AirspaceCatalog::default())),
        at,
    );
    assert_eq!(core.apply(GetAirspaceSnapshot, at).response.generation, 2);
    assert_eq!(snapshot.catalog.sources.len(), 1);
}
