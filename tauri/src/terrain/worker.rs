use super::Terrain;
use crate::driver::DriverHandle;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::watch;
use updraft_core::{TerrainElevation, Topic};

pub async fn watch_elevation(
    terrain: Arc<Mutex<Terrain>>,
    driver: DriverHandle,
) -> anyhow::Result<()> {
    let mut generations = terrain
        .lock()
        .map_err(|_| anyhow::anyhow!("Terrain lock is poisoned"))?
        .changes
        .subscribe();
    let (sender, mut positions) = watch::channel(None);
    driver.subscribe(Box::new(move |topic| {
        if let Topic::Instruments(instruments) = topic {
            let position = instruments.gps.map(|gps| gps.position);
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
        loop {
            generations.borrow_and_update();
            let Some(position) = *positions.borrow_and_update() else {
                break;
            };
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
                continue;
            }
            let (meters, retry) = match result {
                Ok(meters) => (meters, false),
                Err(error) => {
                    tracing::warn!(%error, "Could not sample terrain elevation");
                    (None, true)
                }
            };
            driver.send(TerrainElevation { position, meters }).await?;
            if !retry {
                break;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
    Ok(())
}
