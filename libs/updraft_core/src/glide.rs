use crate::{ArrivalReserve, MacCready, WaypointSnapshot, topic::Instruments};
use updraft_geo::{BoundingBox, LatLon};
use updraft_polar::GlidePolar;
use updraft_units::{Angle, Length, Speed};
use updraft_waypoint::{Waypoint, WaypointKind};

/// Waypoints and glide inputs captured by one core query.
/// Later sensor, catalog, and settings changes do not alter this snapshot.
#[derive(Clone, Debug)]
pub struct GlideSnapshot {
    pub waypoints: WaypointSnapshot,
    pub instruments: Instruments,
    pub polar: GlidePolar,
    pub mac_cready: MacCready,
    pub arrival_reserve: ArrivalReserve,
}

/// Arrival height above field elevation and reserve, without terrain clearance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaypointArrival {
    pub margin: Length,
    pub stale: bool,
}

/// Results for selected landables, identified within one catalog generation.
#[derive(Clone, Debug, PartialEq)]
pub struct WaypointArrivals {
    pub generation: u64,
    pub entries: Vec<WaypointArrivalEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaypointArrivalEntry {
    pub source_index: usize,
    pub waypoint_index: usize,
    pub arrival: Option<WaypointArrival>,
}

impl GlideSnapshot {
    /// Calculates all landables within the viewport plus 10% on each side.
    /// Padding uses latitude and longitude spans. The viewport must follow `BoundingBox`'s contract.
    /// Indices refer to the full catalog, including unavailable sources and non-landables.
    /// Selected waypoints without a solution remain in the batch with `arrival: None`.
    pub fn arrivals_in(&self, viewport: BoundingBox) -> WaypointArrivals {
        let bounds = buffered_viewport(viewport);
        let mut entries = Vec::new();
        for (source_index, source) in self.waypoints.catalog.sources.values().enumerate() {
            let Ok(dataset) = source else { continue };
            for (waypoint_index, waypoint) in dataset.waypoints().iter().enumerate() {
                let landable = matches!(
                    waypoint.kind,
                    WaypointKind::GrassAirfield
                        | WaypointKind::SolidAirfield
                        | WaypointKind::GlidingAirfield
                        | WaypointKind::Outlanding
                );
                if landable && bounds.contains(waypoint.position) {
                    entries.push(WaypointArrivalEntry {
                        source_index,
                        waypoint_index,
                        arrival: self.arrival_at(waypoint),
                    });
                }
            }
        }
        WaypointArrivals {
            generation: self.waypoints.generation,
            entries,
        }
    }

    /// Calculates a direct arrival using fused altitude and the available wind, or calm air.
    /// Stale position or altitude marks the result stale. Stale wind remains usable.
    /// Returns `None` for missing position or fused altitude, or an unsolvable glide.
    pub fn arrival_at(&self, waypoint: &Waypoint) -> Option<WaypointArrival> {
        let gps = self.instruments.gps?;
        let derived = self.instruments.derived.as_ref()?;
        let altitude = derived.altitude?;
        let position = LatLon::from_degrees(
            gps.position.latitude_degrees,
            gps.position.longitude_degrees,
        );
        let (distance, bearing) = position.distance_bearing(waypoint.position);
        let (tailwind, crosswind) = derived.wind.map_or((0., 0.), |wind| {
            let angle = wind.direction_degrees.to_radians() - bearing.as_radians();
            let speed = wind.speed_meters_per_second;
            (-speed * angle.cos(), speed * angle.sin())
        });
        let glide = self.polar.solve_glide(
            distance,
            Length::from_meters(altitude.altitude_msl_meters),
            Speed::from_meters_per_second(self.mac_cready.meters_per_second()),
            Speed::from_meters_per_second(tailwind),
            Speed::from_meters_per_second(crosswind),
        )?;
        let margin = altitude.altitude_msl_meters
            - glide.height_loss.as_meters()
            - waypoint.elevation.into_inner().as_meters()
            - self.arrival_reserve.meters();
        Some(WaypointArrival {
            margin: Length::from_meters(margin),
            stale: gps.stale || altitude.stale,
        })
    }
}

fn buffered_viewport(viewport: BoundingBox) -> BoundingBox {
    let latitude_margin = viewport.latitude_span().as_degrees() * 0.1;
    let south = (viewport.south().as_degrees() - latitude_margin).max(-90.);
    let north = (viewport.north().as_degrees() + latitude_margin).min(90.);
    let longitude_span = viewport.longitude_span();
    let (west, east) = if longitude_span.as_degrees() * 1.2 >= 360. {
        (Angle::from_degrees(-180.), Angle::from_degrees(180.))
    } else {
        let margin = longitude_span * 0.1;
        (
            (viewport.west() - margin).normalized_signed(),
            (viewport.east() + margin).normalized_signed(),
        )
    };
    BoundingBox::new(
        Angle::from_degrees(south),
        Angle::from_degrees(north),
        west,
        east,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topic::{DerivedAltitudeInstruments, DerivedInstruments, DerivedWindInstruments};
    use crate::{Core, Fix, GetGlideSnapshot, InternalGps, SettingsSnapshot, Timestamp};
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_ok, assert_some};
    use updraft_units::MslAltitude;
    use updraft_waypoint::WaypointDataset;

    fn snapshot() -> GlideSnapshot {
        let mut core = Core::new(SettingsSnapshot::default());
        let at = Timestamp::from_millis(0);
        let fix = Fix {
            position: LatLon::from_degrees(0., 0.),
            altitude_ellipsoid: None,
            track: None,
            ground_speed: None,
            fix_time: None,
        };
        core.apply(InternalGps::new(fix), at);
        let mut snapshot = core.apply(GetGlideSnapshot, at).response;
        snapshot.instruments.derived = Some(Box::new(DerivedInstruments {
            altitude: Some(DerivedAltitudeInstruments {
                altitude_msl_meters: 1000.,
                stale: false,
            }),
            wind: None,
            raw_vertical_speed: None,
            vertical_speed: None,
            vario: None,
            airspeed: None,
            heading: None,
            bank: None,
            netto: None,
        }));
        snapshot
    }

    fn waypoint() -> Waypoint {
        let cup = b"name,code,country,lat,lon,elev,style\nField,,,0000.000N,00006.000E,100m,2\n";
        let dataset = assert_ok!(WaypointDataset::from_cup(cup));
        dataset.waypoints()[0].clone()
    }

    #[test]
    fn arrival_subtracts_glide_loss_field_elevation_and_reserve() {
        let mut snapshot = snapshot();
        let mut waypoint = waypoint();
        let distance = LatLon::from_degrees(0., 0.).distance(waypoint.position);
        let loss = distance.as_meters() / snapshot.polar.best_glide_ratio();
        let arrival = assert_some!(snapshot.arrival_at(&waypoint));
        assert_abs_diff_eq!(
            arrival.margin.as_meters(),
            1000. - loss - 100. - 200.,
            epsilon = 0.001
        );
        assert!(!arrival.stale);
        waypoint.elevation = MslAltitude::new(Length::from_meters(2000.));
        let below = assert_some!(snapshot.arrival_at(&waypoint));
        assert_eq!(below.margin, arrival.margin - Length::from_meters(1900.));
        snapshot.arrival_reserve = assert_ok!(ArrivalReserve::try_from(250.));
        let reserved = assert_some!(snapshot.arrival_at(&waypoint));
        assert_eq!(reserved.margin, below.margin - Length::from_meters(50.));
    }

    #[test]
    fn wind_projection_uses_target_bearing_and_selected_mc() {
        let mut snapshot = snapshot();
        snapshot.mac_cready = assert_ok!(MacCready::try_from(2.));
        let waypoint = waypoint();
        let distance = LatLon::from_degrees(0., 0.).distance(waypoint.position);
        for (direction, tailwind, crosswind) in [(90., -10., 0.), (270., 10., 0.), (0., 0., 10.)] {
            let derived = assert_some!(snapshot.instruments.derived.as_mut());
            derived.wind = Some(DerivedWindInstruments {
                direction_degrees: direction,
                speed_meters_per_second: 10.,
                stale: true,
            });
            let expected = snapshot.polar.solve_glide(
                distance,
                Length::from_meters(1000.),
                Speed::from_meters_per_second(2.),
                Speed::from_meters_per_second(tailwind),
                Speed::from_meters_per_second(crosswind),
            );
            let expected = assert_some!(expected);
            let arrival = assert_some!(snapshot.arrival_at(&waypoint));
            assert_abs_diff_eq!(
                arrival.margin.as_meters(),
                700. - expected.height_loss.as_meters(),
                epsilon = 1e-9
            );
            assert!(!arrival.stale);
        }
    }

    #[test]
    fn stale_position_or_altitude_retains_the_margin() {
        let mut snapshot = snapshot();
        let waypoint = waypoint();
        let fresh = assert_some!(snapshot.arrival_at(&waypoint));
        for (position_stale, altitude_stale) in [(true, false), (false, true), (true, true)] {
            assert_some!(snapshot.instruments.gps.as_mut()).stale = position_stale;
            let derived = assert_some!(snapshot.instruments.derived.as_mut());
            assert_some!(derived.altitude.as_mut()).stale = altitude_stale;
            let arrival = assert_some!(snapshot.arrival_at(&waypoint));
            assert_eq!(arrival.margin, fresh.margin);
            assert!(arrival.stale);
        }
    }

    #[test]
    fn missing_inputs_and_impossible_wind_have_no_arrival() {
        let waypoint = waypoint();
        let mut missing_position = snapshot();
        missing_position.instruments.gps = None;
        assert_none!(missing_position.arrival_at(&waypoint));
        let mut missing_altitude = snapshot();
        let derived = assert_some!(missing_altitude.instruments.derived.as_mut());
        derived.altitude = None;
        let gps = assert_some!(missing_altitude.instruments.gps.as_mut());
        gps.altitude_meters = Some(1000.);
        assert_none!(missing_altitude.arrival_at(&waypoint));
        let mut missing_fusion = snapshot();
        missing_fusion.instruments.derived = None;
        assert_none!(missing_fusion.arrival_at(&waypoint));
        let mut impossible = snapshot();
        let derived = assert_some!(impossible.instruments.derived.as_mut());
        derived.wind = Some(DerivedWindInstruments {
            direction_degrees: 90.,
            speed_meters_per_second: 120.,
            stale: false,
        });
        assert_none!(impossible.arrival_at(&waypoint));
    }

    #[test]
    fn viewport_batch_keeps_catalog_identity_and_only_selects_landables() {
        use crate::{WaypointCatalog, WaypointLoadError};
        use std::{collections::BTreeMap, sync::Arc};

        let mut cup = String::from("name,code,country,lat,lon,elev,style\n");
        for kind in 0..=21 {
            cup.push_str(&format!("Field,,,0000.000N,00006.000E,100m,{kind}\n"));
        }
        cup.push_str("Outside,,,1000.000N,01000.000E,100m,2\n");
        let dataset = Arc::new(assert_ok!(WaypointDataset::from_cup(cup.as_bytes())));
        let mut snapshot = snapshot();
        snapshot.waypoints = WaypointSnapshot {
            generation: 42,
            catalog: Arc::new(WaypointCatalog {
                sources: BTreeMap::from([
                    ("a.cup".into(), Err(WaypointLoadError::ReadFailed)),
                    ("b.cup".into(), Ok(dataset.clone())),
                    ("c.cup".into(), Ok(dataset.clone())),
                ]),
            }),
        };
        let viewport = bounds(-1., 1., -1., 1.);
        let batch = snapshot.arrivals_in(viewport);
        assert_eq!(batch.generation, 42);
        let identities: Vec<_> = batch
            .entries
            .iter()
            .map(|entry| (entry.source_index, entry.waypoint_index))
            .collect();
        let expected_ids = [
            (1, 2),
            (1, 3),
            (1, 4),
            (1, 5),
            (2, 2),
            (2, 3),
            (2, 4),
            (2, 5),
        ];
        assert_eq!(identities, expected_ids);
        let expected = assert_some!(snapshot.arrival_at(&dataset.waypoints()[2]));
        for entry in batch.entries {
            assert_eq!(assert_some!(entry.arrival), expected);
        }
        snapshot.instruments.gps = None;
        let unavailable = snapshot.arrivals_in(viewport);
        assert_eq!(unavailable.entries.len(), 8);
        for entry in unavailable.entries {
            assert_none!(entry.arrival);
        }
        let empty = snapshot.arrivals_in(bounds(20., 21., 20., 21.));
        assert_eq!(empty.entries.len(), 0);
    }

    fn bounds(south: f64, north: f64, west: f64, east: f64) -> BoundingBox {
        BoundingBox::new(
            Angle::from_degrees(south),
            Angle::from_degrees(north),
            Angle::from_degrees(west),
            Angle::from_degrees(east),
        )
    }

    #[test]
    fn viewport_buffer_expands_each_side_and_handles_global_edges() {
        for (viewport, expected) in [
            (bounds(0., 10., 0., 10.), bounds(-1., 11., -1., 11.)),
            (
                bounds(-10., 10., 170., -170.),
                bounds(-12., 12., 168., -168.),
            ),
            (
                bounds(-89., 89., -170., 170.),
                bounds(-90., 90., -180., 180.),
            ),
            (bounds(1., 1., 2., 2.), bounds(1., 1., 2., 2.)),
        ] {
            let buffered = buffered_viewport(viewport);
            for (actual, expected) in [
                (buffered.south(), expected.south()),
                (buffered.north(), expected.north()),
                (buffered.west(), expected.west()),
                (buffered.east(), expected.east()),
            ] {
                assert_abs_diff_eq!(actual.as_degrees(), expected.as_degrees(), epsilon = 1e-12);
            }
        }
        let mut snapshot = snapshot();
        let waypoint = waypoint();
        use std::{collections::BTreeMap, sync::Arc};
        let cup = b"name,code,country,lat,lon,elev,style\nField,,,0000.000N,00006.000E,100m,2\n";
        let dataset = Arc::new(assert_ok!(WaypointDataset::from_cup(cup)));
        snapshot.waypoints.catalog = Arc::new(crate::WaypointCatalog {
            sources: BTreeMap::from([("field.cup".into(), Ok(dataset))]),
        });
        let viewport = bounds(-1., 1., -1., 0.);
        assert!(!viewport.contains(waypoint.position));
        assert_eq!(snapshot.arrivals_in(viewport).entries.len(), 1);
        let outside_buffer = snapshot.arrivals_in(bounds(-1., 1., -1., -0.01));
        assert_eq!(outside_buffer.entries.len(), 0);
    }
}
