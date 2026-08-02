use super::super::*;
use super::support::at;
use crate::{AirspaceLoadError, AirspaceStatus};
use claims::assert_some;
use std::sync::Arc;

const POLYGON: &[u8] = include_bytes!("../../tests/fixtures/airspace/polygon.txt");

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

    assert_eq!(airspace_status(&core), AirspaceStatus::None);
}

#[test]
fn activating_airspace_dataset_publishes_and_onboards_active_status() {
    let mut core = Core::new(SettingsSnapshot::default());
    let dataset = Arc::new(AirspaceDataset::from_openair(POLYGON).expect("a valid fixture"));

    let update = core.apply(
        ActivateAirspaceDataset::new(dataset, Some("Local airspace.txt".into())),
        at(0),
    );

    let status = AirspaceStatus::Active {
        source_name: Some("Local airspace.txt".into()),
        airspace_count: 1,
        generation: 1,
    };
    assert_eq!(
        update.effects,
        vec![Effect::emit(Topic::Airspace(status.clone()))]
    );
    assert_eq!(airspace_status(&core), status);
}

#[test]
fn airspace_generation_changes_only_for_a_different_dataset() {
    let mut core = Core::new(SettingsSnapshot::default());
    let dataset = Arc::new(AirspaceDataset::from_openair(POLYGON).expect("a valid fixture"));
    core.apply(
        ActivateAirspaceDataset::new(dataset.clone(), Some("First name".into())),
        at(0),
    );

    core.apply(
        ActivateAirspaceDataset::new(dataset, Some("Updated name".into())),
        at(1),
    );

    assert_eq!(
        airspace_status(&core),
        AirspaceStatus::Active {
            source_name: Some("Updated name".into()),
            airspace_count: 1,
            generation: 1,
        }
    );

    let replacement = Arc::new(AirspaceDataset::default());
    core.apply(
        ActivateAirspaceDataset::new(replacement, Some("Empty source".into())),
        at(2),
    );

    assert_eq!(
        airspace_status(&core),
        AirspaceStatus::Active {
            source_name: Some("Empty source".into()),
            airspace_count: 0,
            generation: 2,
        }
    );
}

#[test]
fn airspace_generation_advances_across_removal() {
    let mut core = Core::new(SettingsSnapshot::default());
    let dataset = Arc::new(AirspaceDataset::from_openair(POLYGON).expect("a valid fixture"));
    core.apply(ActivateAirspaceDataset::new(dataset.clone(), None), at(0));
    core.apply(ClearAirspaceDataset, at(1));

    core.apply(ActivateAirspaceDataset::new(dataset, None), at(2));

    assert_eq!(
        airspace_status(&core),
        AirspaceStatus::Active {
            source_name: None,
            airspace_count: 1,
            generation: 3,
        }
    );
}

#[test]
fn clearing_airspace_dataset_publishes_none_status() {
    let mut core = Core::new(SettingsSnapshot::default());
    let dataset = Arc::new(AirspaceDataset::from_openair(POLYGON).expect("a valid fixture"));
    core.apply(
        ActivateAirspaceDataset::new(dataset, Some("Local airspace.txt".into())),
        at(0),
    );

    let update = core.apply(ClearAirspaceDataset, at(1));

    assert_eq!(
        update.effects,
        vec![Effect::emit(Topic::Airspace(AirspaceStatus::None))]
    );
    assert_eq!(airspace_status(&core), AirspaceStatus::None);
}

#[test]
fn setting_airspace_unavailable_publishes_safe_load_error() {
    let mut core = Core::new(SettingsSnapshot::default());

    let update = core.apply(
        SetAirspaceUnavailable::new(
            Some("Broken airspace.txt".into()),
            AirspaceLoadError::ParseFailed,
        ),
        at(0),
    );

    let status = AirspaceStatus::Unavailable {
        source_name: Some("Broken airspace.txt".into()),
        error: AirspaceLoadError::ParseFailed,
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
    core.apply(ActivateAirspaceDataset::new(dataset.clone(), None), at(0));

    let snapshot = assert_some!(core.apply(GetAirspaceSnapshot, at(1)).response);

    assert!(Arc::ptr_eq(&snapshot, &dataset));
}
