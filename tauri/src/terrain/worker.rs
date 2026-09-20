use super::Terrain;
use crate::driver::DriverHandle;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::watch;
use updraft_core::{NavigationElevation, NavigationTarget, TerrainElevation, Topic};

pub async fn watch_elevation(
    terrain: Arc<Mutex<Terrain>>,
    driver: DriverHandle,
) -> anyhow::Result<()> {
    tokio::try_join!(
        watch_position(terrain.clone(), driver.clone(), ElevationTarget::Ownship),
        watch_position(terrain.clone(), driver.clone(), ElevationTarget::Navigation),
        watch_position(terrain, driver, ElevationTarget::Pins),
    )?;
    Ok(())
}

#[derive(Clone, Copy)]
enum ElevationTarget {
    Ownship,
    Navigation,
    Pins,
}

async fn watch_position(
    terrain: Arc<Mutex<Terrain>>,
    driver: DriverHandle,
    target: ElevationTarget,
) -> anyhow::Result<()> {
    let mut generations = terrain
        .lock()
        .map_err(|_| anyhow::anyhow!("Terrain lock is poisoned"))?
        .changes
        .subscribe();
    let (sender, mut positions) = watch::channel(Vec::new());
    driver.subscribe(Box::new(move |topic| {
        let position = match (target, topic) {
            (ElevationTarget::Ownship, Topic::Instruments(instruments)) => Some(
                instruments
                    .gps
                    .map(|gps| (None, gps.position))
                    .into_iter()
                    .collect::<Vec<_>>(),
            ),
            (ElevationTarget::Navigation, Topic::Navigation(navigation)) => Some(
                navigation
                    .as_ref()
                    .and_then(|navigation| {
                        matches!(navigation.target, NavigationTarget::MapPosition { .. })
                            .then_some(navigation.position)
                            .flatten()
                    })
                    .map(|position| (None, position))
                    .into_iter()
                    .collect(),
            ),
            (ElevationTarget::Pins, Topic::PinnedTargets(pins)) => Some(
                pins.iter()
                    .filter_map(|pin| {
                        matches!(pin.navigation.target, NavigationTarget::MapPosition { .. })
                            .then(|| {
                                pin.navigation
                                    .position
                                    .map(|position| (Some(pin.id), position))
                            })
                            .flatten()
                    })
                    .collect(),
            ),
            _ => None,
        };
        if let Some(position) = position {
            sender.send_if_modified(|previous| {
                if *previous == position {
                    return false;
                }
                *previous = position;
                true
            });
        }
        !sender.is_closed()
    }));
    loop {
        tokio::select! {
            changed = positions.changed() => { if changed.is_err() { break } }
            changed = generations.changed() => { if changed.is_err() { break } }
        }
        'sample: loop {
            generations.borrow_and_update();
            let targets = positions.borrow_and_update().clone();
            let mut retry_any = false;
            for (id, position) in targets {
                let terrain = terrain.clone();
                let result = tokio::task::spawn_blocking(move || {
                    terrain
                        .lock()
                        .map_err(|_| anyhow::anyhow!("Terrain lock is poisoned"))?
                        .reader
                        .elevation(updraft_geo::LatLon::from_degrees(
                            position.latitude_degrees,
                            position.longitude_degrees,
                        ))
                })
                .await
                .map_err(anyhow::Error::from)
                .and_then(std::convert::identity);
                if generations.has_changed()? {
                    continue 'sample;
                }
                let (meters, retry) = match result {
                    Ok(meters) => (meters, false),
                    Err(error) => {
                        tracing::warn!(%error, "Could not sample terrain elevation");
                        (None, true)
                    }
                };
                match target {
                    ElevationTarget::Ownship => {
                        driver.send(TerrainElevation { position, meters }).await?
                    }
                    ElevationTarget::Navigation => {
                        driver
                            .send(NavigationElevation { position, meters })
                            .await?
                    }
                    ElevationTarget::Pins => {
                        if let Some(id) = id {
                            driver
                                .send(updraft_core::PinnedTargetElevation {
                                    id,
                                    position,
                                    meters,
                                })
                                .await?;
                        }
                    }
                }
                retry_any |= retry;
            }
            if !retry_any {
                break;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
    Ok(())
}
