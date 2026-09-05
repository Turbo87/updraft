use super::arrival_resource::ArrivalResource;
use crate::driver::DriverHandle;
use std::{future::pending, time::Duration};
use tokio::{sync::watch, task::JoinHandle, time::Instant};
use updraft_core::{GetGlideSnapshot, Topic};
use updraft_geo::BoundingBox;

/// One viewport's arrival worker. Drop the viewport sender to stop it.
/// The owner must observe `task` for driver or calculation failures.
pub struct ArrivalCalculator {
    pub viewport: watch::Sender<BoundingBox>,
    pub results: watch::Receiver<Option<ArrivalResource>>,
    pub task: JoinHandle<anyhow::Result<()>>,
}

impl ArrivalCalculator {
    pub fn spawn(driver: DriverHandle, bounds: BoundingBox) -> Self {
        let (viewport, mut viewports) = watch::channel(bounds);
        let (result_sender, results) = watch::channel(None);
        let (input_sender, mut inputs) = watch::channel(());
        driver.subscribe(Box::new(move |topic| {
            if matches!(
                topic,
                Topic::Instruments(_)
                    | Topic::Settings(_)
                    | Topic::GlidePerformance(_)
                    | Topic::Waypoints(_)
            ) {
                input_sender.send(()).is_ok()
            } else {
                !input_sender.is_closed()
            }
        }));
        let task = tokio::spawn(async move {
            let mut last_run = Instant::now() - Duration::from_secs(1);
            let mut next_run = Some(Instant::now());
            loop {
                let delay = async {
                    match next_run {
                        Some(at) => tokio::time::sleep_until(at).await,
                        None => pending().await,
                    }
                };
                tokio::select! {
                    changed = viewports.changed() => {
                        if changed.is_err() { return Ok(()) }
                        let at = last_run + Duration::from_millis(100);
                        next_run = Some(next_run.map_or(at, |pending| pending.min(at)));
                    }
                    changed = inputs.changed() => {
                        changed?;
                        let at = last_run + Duration::from_secs(1);
                        next_run = Some(next_run.map_or(at, |pending| pending.min(at)));
                    }
                    () = delay => {
                        next_run = None;
                        let bounds = *viewports.borrow_and_update();
                        inputs.borrow_and_update();
                        last_run = Instant::now();
                        let snapshot = driver.send(GetGlideSnapshot).await?;
                        let calculate = move || ArrivalResource::calculate(&snapshot, bounds);
                        let arrivals = tokio::task::spawn_blocking(calculate).await??;
                        if result_sender.send(Some(arrivals)).is_err() { return Ok(()) }
                    }
                }
            }
        });
        Self {
            viewport,
            results,
            task,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::tests::spawn;
    use claims::{assert_err, assert_lt, assert_none, assert_ok, assert_some};
    use std::{collections::BTreeMap, sync::Arc};
    use updraft_core::{
        Fix, Input, InternalGps, MacCready, PolarId, ReplaceWaypointCatalog, SetArrivalReserve,
        SetBallast, SetBugs, SetMacCready, SetPolar, SettingsSnapshot, WaypointCatalog,
    };
    use updraft_geo::LatLon;
    use updraft_units::{Angle, EllipsoidAltitude, Length};
    use updraft_waypoint::WaypointDataset;

    fn feature_count(results: &watch::Receiver<Option<ArrivalResource>>) -> usize {
        let resource = results.borrow();
        let resource = assert_some!(resource.as_ref());
        let geojson: serde_json::Value = assert_ok!(serde_json::from_slice(&resource.body));
        assert_some!(geojson["features"].as_array()).len()
    }

    fn arrival_margin(results: &watch::Receiver<Option<ArrivalResource>>) -> f64 {
        let resource = results.borrow();
        let resource = assert_some!(resource.as_ref());
        let geojson: serde_json::Value = assert_ok!(serde_json::from_slice(&resource.body));
        assert_some!(geojson["features"][0]["properties"]["arrivalMarginMeters"].as_f64())
    }

    async fn updated_margin(
        driver: &DriverHandle,
        results: &mut watch::Receiver<Option<ArrivalResource>>,
        input: impl Input<Response = ()>,
    ) -> f64 {
        assert_ok!(driver.send(input).await);
        let changed = tokio::time::timeout(Duration::from_secs(2), results.changed()).await;
        assert_ok!(assert_ok!(changed));
        arrival_margin(results)
    }

    #[tokio::test(start_paused = true)]
    async fn worker_throttles_updates_and_retains_the_last_result() {
        let driver = spawn(
            SettingsSnapshot::default(),
            Box::new(|_, _, _| unreachable!()),
            Box::new(|_| {}),
            Duration::from_secs(60),
        );
        let bounds = BoundingBox::new(Angle::ZERO, Angle::ZERO, Angle::ZERO, Angle::ZERO);
        let cup = b"name,code,country,lat,lon,elev,style\nField,,,0000.000N,00000.000E,100m,2\n";
        let dataset = Arc::new(assert_ok!(WaypointDataset::from_cup(cup)));
        let catalog = Arc::new(WaypointCatalog {
            sources: BTreeMap::from([("field.cup".into(), Ok(dataset))]),
        });
        assert_ok!(driver.handle.send(ReplaceWaypointCatalog(catalog)).await);
        let fix = Fix {
            position: LatLon::from_degrees(0., 0.1),
            altitude_ellipsoid: Some(EllipsoidAltitude::new(Length::from_meters(1000.))),
            track: None,
            ground_speed: None,
            fix_time: None,
        };
        assert_ok!(driver.handle.send(InternalGps::new(fix)).await);
        let ArrivalCalculator {
            viewport,
            mut results,
            task,
        } = ArrivalCalculator::spawn(driver.handle.clone(), bounds);
        assert_ok!(results.changed().await);
        assert_eq!(feature_count(&results), 1);
        let initial_margin = arrival_margin(&results);
        let first = Instant::now();
        viewport.send_replace(bounds);
        viewport.send_replace(bounds);
        assert_ok!(results.changed().await);
        assert_eq!(Instant::now() - first, Duration::from_millis(100));
        let second = Instant::now();
        let mac_cready = assert_ok!(MacCready::try_from(1.));
        assert_ok!(driver.handle.send(SetMacCready { mac_cready }).await);
        assert_some!(results.borrow().as_ref());
        assert_ok!(results.changed().await);
        assert_eq!(Instant::now() - second, Duration::from_secs(1));
        let mc_margin = arrival_margin(&results);
        assert_lt!(mc_margin, initial_margin);
        let bugs = assert_ok!(10.0.try_into());
        let bugs_margin = updated_margin(&driver.handle, &mut results, SetBugs { bugs }).await;
        assert_lt!(bugs_margin, mc_margin);
        let ballast = assert_ok!(100.0.try_into());
        let ballast_margin =
            updated_margin(&driver.handle, &mut results, SetBallast { ballast }).await;
        assert_ne!(ballast_margin, bugs_margin);
        let polar = assert_ok!(PolarId::try_from("JS-3-18m".to_owned()));
        let polar_margin = updated_margin(&driver.handle, &mut results, SetPolar { polar }).await;
        assert_ne!(polar_margin, ballast_margin);
        let reserve = assert_ok!(300.0.try_into());
        let reserve_margin =
            updated_margin(&driver.handle, &mut results, SetArrivalReserve { reserve }).await;
        assert_eq!(reserve_margin, polar_margin - 100.);
        let third = Instant::now();
        let mac_cready = assert_ok!(MacCready::try_from(2.));
        assert_ok!(driver.handle.send(SetMacCready { mac_cready }).await);
        tokio::time::advance(Duration::from_millis(50)).await;
        let elsewhere = Angle::from_degrees(10.);
        viewport.send_replace(bounds);
        viewport.send_replace(BoundingBox::new(elsewhere, elsewhere, elsewhere, elsewhere));
        assert_ok!(results.changed().await);
        assert_eq!(Instant::now() - third, Duration::from_millis(100));
        assert_eq!(feature_count(&results), 0);
        tokio::time::advance(Duration::from_secs(5)).await;
        tokio::task::yield_now().await;
        assert!(!assert_ok!(results.has_changed()));
        drop(viewport);
        assert_ok!(assert_ok!(task.await));
        driver.terminate().await;
    }

    #[tokio::test]
    async fn stopped_driver_fails_the_worker_without_publishing_a_result() {
        let bounds = BoundingBox::new(Angle::ZERO, Angle::ZERO, Angle::ZERO, Angle::ZERO);
        let worker = ArrivalCalculator::spawn(DriverHandle::stopped(), bounds);
        assert_err!(assert_ok!(worker.task.await));
        assert_none!(worker.results.borrow().as_ref());
    }
}
