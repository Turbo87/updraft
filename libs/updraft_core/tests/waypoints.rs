use claims::assert_ok;
use std::{collections::BTreeMap, sync::Arc};
use updraft_core::{
    Core, GetWaypointCatalog, ReplaceWaypointCatalog, SettingsSnapshot, Timestamp, Topic,
    WaypointCatalog,
};
use updraft_waypoint::WaypointDataset;

#[test]
fn replaces_catalog_and_publishes_a_generation_without_merging_sources() {
    let bytes = b"name,code,country,lat,lon,elev,style\nField,,,5000.000N,00600.000E,100m,2\n";
    let dataset = Arc::new(assert_ok!(WaypointDataset::from_cup(bytes)));
    let catalog = Arc::new(WaypointCatalog {
        sources: BTreeMap::from([
            ("a.cup".into(), Ok(dataset.clone())),
            ("b.cup".into(), Ok(dataset)),
        ]),
    });
    let mut core = Core::new(SettingsSnapshot::default());
    let at = Timestamp::from_millis(0);
    core.apply(ReplaceWaypointCatalog(catalog.clone()), at);
    let snapshot = core.apply(GetWaypointCatalog, at).response;
    assert!(Arc::ptr_eq(&snapshot, &catalog));
    let status = core
        .topics()
        .into_iter()
        .find_map(|topic| match topic {
            Topic::Waypoints(status) => Some(status),
            _ => None,
        })
        .unwrap();
    assert_eq!(status.generation, 1);
    assert_eq!(status.sources.len(), 2);
    core.apply(
        ReplaceWaypointCatalog(Arc::new(WaypointCatalog::default())),
        at,
    );
    assert!(
        core.apply(GetWaypointCatalog, at)
            .response
            .sources
            .is_empty()
    );
    assert_eq!(catalog.sources.len(), 2);
}
