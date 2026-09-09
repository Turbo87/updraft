use super::super::*;
use super::support::at;
use crate::{AirspaceLoadError, AirspaceSource, AirspaceStatus};
use std::{collections::BTreeMap, sync::Arc};
use updraft_airspace::AirspaceDataset;

const POLYGON: &[u8] = include_bytes!("../../../../testdata/airspace/polygon.txt");

fn airspace_status(core: &Core) -> AirspaceStatus {
    core.topics()
        .into_iter()
        .find_map(|topic| {
            let Topic::Airspace(status) = topic else {
                return None;
            };
            Some(status)
        })
        .expect("an airspace status topic")
}

#[test]
fn new_core_has_no_airspace() {
    let core = Core::new(SettingsSnapshot::default());

    assert_eq!(airspace_status(&core), AirspaceStatus::default());
}

#[test]
fn activating_airspace_dataset_publishes_and_onboards_active_status() {
    let mut core = Core::new(SettingsSnapshot::default());
    let dataset = Arc::new(AirspaceDataset::from_openair(POLYGON).expect("a valid fixture"));

    let update = core.apply(
        crate::ReplaceAirspaceCatalog(Arc::new(crate::AirspaceCatalog {
            sources: BTreeMap::from([(
                "Local airspace.txt".into(),
                AirspaceSource::Active(dataset),
            )]),
        })),
        at(0),
    );

    let status = AirspaceStatus {
        generation: 1,
        sources: vec![crate::AirspaceSourceStatus::Active {
            source_name: "Local airspace.txt".into(),
            airspace_count: 1,
        }],
    };
    assert_eq!(
        update.effects,
        vec![Effect::emit(Topic::Airspace(status.clone()))]
    );
    assert_eq!(airspace_status(&core), status);
}

#[test]
fn airspace_generation_changes_for_each_replacement() {
    let mut core = Core::new(SettingsSnapshot::default());
    let dataset = Arc::new(AirspaceDataset::from_openair(POLYGON).expect("a valid fixture"));
    core.apply(
        crate::ReplaceAirspaceCatalog(Arc::new(crate::AirspaceCatalog {
            sources: BTreeMap::from([(
                "First name".into(),
                AirspaceSource::Active(dataset.clone()),
            )]),
        })),
        at(0),
    );

    core.apply(
        crate::ReplaceAirspaceCatalog(Arc::new(crate::AirspaceCatalog {
            sources: BTreeMap::from([("Updated name".into(), AirspaceSource::Active(dataset))]),
        })),
        at(1),
    );

    assert_eq!(
        airspace_status(&core),
        AirspaceStatus {
            generation: 2,
            sources: vec![crate::AirspaceSourceStatus::Active {
                source_name: "Updated name".into(),
                airspace_count: 1
            }]
        }
    );

    let replacement = Arc::new(AirspaceDataset::default());
    core.apply(
        crate::ReplaceAirspaceCatalog(Arc::new(crate::AirspaceCatalog {
            sources: BTreeMap::from([("Empty source".into(), AirspaceSource::Active(replacement))]),
        })),
        at(2),
    );

    assert_eq!(
        airspace_status(&core),
        AirspaceStatus {
            generation: 3,
            sources: vec![crate::AirspaceSourceStatus::Active {
                source_name: "Empty source".into(),
                airspace_count: 0
            }]
        }
    );
}

#[test]
fn airspace_generation_advances_across_removal() {
    let mut core = Core::new(SettingsSnapshot::default());
    let dataset = Arc::new(AirspaceDataset::from_openair(POLYGON).expect("a valid fixture"));
    core.apply(
        crate::ReplaceAirspaceCatalog(Arc::new(crate::AirspaceCatalog {
            sources: BTreeMap::from([(
                "airspace.txt".into(),
                AirspaceSource::Active(dataset.clone()),
            )]),
        })),
        at(0),
    );
    core.apply(crate::ReplaceAirspaceCatalog(Arc::default()), at(1));

    core.apply(
        crate::ReplaceAirspaceCatalog(Arc::new(crate::AirspaceCatalog {
            sources: BTreeMap::from([("airspace.txt".into(), AirspaceSource::Active(dataset))]),
        })),
        at(2),
    );

    assert_eq!(
        airspace_status(&core),
        AirspaceStatus {
            generation: 3,
            sources: vec![crate::AirspaceSourceStatus::Active {
                source_name: "airspace.txt".into(),
                airspace_count: 1
            }]
        }
    );
}

#[test]
fn clearing_airspace_dataset_publishes_none_status() {
    let mut core = Core::new(SettingsSnapshot::default());
    let dataset = Arc::new(AirspaceDataset::from_openair(POLYGON).expect("a valid fixture"));
    core.apply(
        crate::ReplaceAirspaceCatalog(Arc::new(crate::AirspaceCatalog {
            sources: BTreeMap::from([(
                "Local airspace.txt".into(),
                AirspaceSource::Active(dataset),
            )]),
        })),
        at(0),
    );

    let update = core.apply(crate::ReplaceAirspaceCatalog(Arc::default()), at(1));

    assert_eq!(
        update.effects,
        vec![Effect::emit(Topic::Airspace(AirspaceStatus {
            generation: 2,
            sources: vec![]
        }))]
    );
    assert_eq!(
        airspace_status(&core),
        AirspaceStatus {
            generation: 2,
            sources: vec![]
        }
    );
}

#[test]
fn setting_airspace_unavailable_publishes_safe_load_error() {
    let mut core = Core::new(SettingsSnapshot::default());

    let update = core.apply(
        crate::ReplaceAirspaceCatalog(Arc::new(crate::AirspaceCatalog {
            sources: BTreeMap::from([(
                "Broken airspace.txt".into(),
                AirspaceSource::Unavailable(AirspaceLoadError::ParseFailed),
            )]),
        })),
        at(0),
    );

    let status = AirspaceStatus {
        generation: 1,
        sources: vec![crate::AirspaceSourceStatus::Unavailable {
            source_name: "Broken airspace.txt".into(),
            error: AirspaceLoadError::ParseFailed,
        }],
    };
    assert_eq!(
        update.effects,
        vec![Effect::emit(Topic::Airspace(status.clone()))]
    );
    assert_eq!(airspace_status(&core), status);
}

#[test]
fn airspace_snapshot_shares_the_immutable_dataset() {
    let mut core = Core::new(SettingsSnapshot::default());
    let dataset = Arc::new(AirspaceDataset::from_openair(POLYGON).expect("a valid fixture"));
    core.apply(
        crate::ReplaceAirspaceCatalog(Arc::new(crate::AirspaceCatalog {
            sources: BTreeMap::from([(
                "airspace.txt".into(),
                AirspaceSource::Active(dataset.clone()),
            )]),
        })),
        at(0),
    );

    let snapshot = core.apply(GetAirspaceSnapshot, at(1)).response;
    let AirspaceSource::Active(snapshot) = &snapshot.catalog.sources["airspace.txt"] else {
        panic!("an active source")
    };

    assert!(Arc::ptr_eq(snapshot, &dataset));
}
