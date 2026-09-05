use crate::{ArrivalReserve, MacCready, WaypointSnapshot, topic::Instruments};
use updraft_geo::LatLon;
use updraft_polar::GlidePolar;
use updraft_units::{Length, Speed};
use updraft_waypoint::Waypoint;

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

impl GlideSnapshot {
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
}
