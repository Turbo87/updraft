use crate::ownship::GpsSnapshot;
use crate::time::Timestamp;
use updraft_air::{AirState, AirStateEstimator, Fix as AirFix};
use updraft_units::{Length, PressureAltitude, Speed};

/// Horizontal accuracy assumed for each unit of dilution of precision,
/// in metres.
///
/// A receiver reports the dilution rather than an accuracy, and the two
/// differ by the error of one range measurement. Five metres suits the
/// consumer receivers this reads from.
const ACCURACY_PER_DILUTION: f64 = 5.;

/// Accuracy assumed where the selected source reports no dilution.
///
/// The internal receiver of a phone reports none, and neither does a
/// device that sends `RMC` without `GGA`. The wind filter only scales
/// its measurement noise with this and never below its own floor, so a
/// pessimistic value costs little.
const DEFAULT_ACCURACY: f64 = 15.;

/// Feeds the air-state estimate from the selected source of each domain.
///
/// Reading the selection rather than the sentences keeps one receiver
/// and one barometer behind the estimate. Two receivers a few metres
/// apart would otherwise share one height filter, and each of their
/// fixes would fold into the wind filter as an independent measurement.
///
/// A selection is re-evaluated on every message, so the same value is
/// offered many times. Each domain therefore remembers the ingestion
/// time it last passed on.
#[derive(Clone, Debug, Default)]
pub struct AirSensors {
    estimator: AirStateEstimator,
    gps_at: Option<Timestamp>,
    pressure_at: Option<Timestamp>,
    air_speed_at: Option<Timestamp>,
}

impl AirSensors {
    /// Takes the selected GPS domain, once per ingestion time.
    ///
    /// The altitude goes on its own call, because the estimator pairs it
    /// with a pressure altitude of the same moment and a ground velocity
    /// without one is still worth having.
    ///
    /// It wants the height above the ellipsoid, because a receiver's own
    /// geoid model is coarse and two receivers do not share one.
    /// Selection carries the height above mean sea level, so EGM96 lifts
    /// it back. What is left is the difference between the two models,
    /// which the filter holds in its offset and not in the vertical
    /// speed.
    pub fn gps(&mut self, at: Timestamp, snapshot: &GpsSnapshot) {
        if self.gps_at == Some(at) {
            return;
        }
        self.gps_at = Some(at);

        if let Some(altitude) = snapshot.altitude_msl {
            let ellipsoid = updraft_egm96::msl_to_ellipsoidal(snapshot.position, altitude);
            self.estimator.gnss_altitude(at.since_start(), ellipsoid);
        }

        let (Some(track), Some(ground_speed)) = (snapshot.track, snapshot.ground_speed) else {
            return;
        };
        self.estimator.fix(
            at.since_start(),
            &AirFix {
                position: snapshot.position,
                track,
                ground_speed,
                position_accuracy: Length::from_meters(
                    snapshot
                        .horizontal_dilution
                        .map_or(DEFAULT_ACCURACY, |dilution| {
                            dilution * ACCURACY_PER_DILUTION
                        }),
                ),
            },
        );
    }

    /// Takes the selected barometric altitude against the 1013.25 hPa
    /// datum, once per ingestion time.
    pub fn pressure_altitude(&mut self, at: Timestamp, altitude: PressureAltitude) {
        if self.pressure_at == Some(at) {
            return;
        }
        self.pressure_at = Some(at);
        self.estimator.pressure_altitude(at.since_start(), altitude);
    }

    /// Takes the selected true airspeed, once per ingestion time.
    pub fn air_speed(&mut self, at: Timestamp, speed: Speed) {
        if self.air_speed_at == Some(at) {
            return;
        }
        self.air_speed_at = Some(at);
        self.estimator.air_speed(at.since_start(), speed);
    }

    pub fn state(&self) -> Option<AirState> {
        self.estimator.state()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};
    use updraft_geo::LatLon;
    use updraft_units::{Angle, MslAltitude};

    fn at(millis: u64) -> Timestamp {
        Timestamp::from_millis(millis)
    }

    fn snapshot(altitude: Option<f64>) -> GpsSnapshot {
        GpsSnapshot {
            position: LatLon::from_degrees(50.8, 6.2),
            altitude_msl: altitude.map(|value| MslAltitude::new(Length::from_meters(value))),
            track: Some(Angle::from_degrees(90.)),
            ground_speed: Some(Speed::from_kilometers_per_hour(120.)),
            fix_time: None,
            horizontal_dilution: None,
        }
    }

    #[test]
    fn a_climb_of_two_metres_per_second_reads_as_one() {
        let mut sensors = AirSensors::default();
        for second in 0..60 {
            let time = at(second * 1_000);
            sensors.pressure_altitude(
                time,
                PressureAltitude::new(Length::from_meters(1000. + 2. * second as f64)),
            );
            sensors.gps(time, &snapshot(None));
        }

        let state = assert_some!(sensors.state());
        let climb = state.vertical_speed.as_meters_per_second();
        assert!((climb - 2.).abs() < 0.01, "climb read {climb} m/s");
    }

    #[test]
    fn a_selected_altitude_gives_the_height_a_sea_level_reference() {
        let mut sensors = AirSensors::default();
        for second in 0..60 {
            let time = at(second * 1_000);
            sensors.pressure_altitude(
                time,
                PressureAltitude::new(Length::from_meters(1000. + 2. * second as f64)),
            );
            sensors.gps(time, &snapshot(Some(500. + 2. * second as f64)));
        }

        // The altitude reported back is the one that went in, because
        // EGM96 undoes the lift onto the ellipsoid exactly.
        let altitude = assert_some!(assert_some!(sensors.state()).altitude);
        let meters = altitude.into_inner().as_meters();
        assert!((meters - 618.).abs() < 1., "altitude read {meters:.1} m");
    }

    #[test]
    fn one_ingestion_time_is_passed_on_once() {
        let mut sensors = AirSensors::default();
        let mut repeated = AirSensors::default();
        for second in 0..60 {
            let time = at(second * 1_000);
            let altitude = PressureAltitude::new(Length::from_meters(1000. + 2. * second as f64));
            sensors.pressure_altitude(time, altitude);
            sensors.gps(time, &snapshot(None));

            // The same selection, offered again as every later message
            // in the same second would.
            repeated.pressure_altitude(time, altitude);
            repeated.gps(time, &snapshot(None));
            repeated.pressure_altitude(time, altitude);
            repeated.gps(time, &snapshot(None));
        }

        let once = assert_some!(sensors.state());
        let twice = assert_some!(repeated.state());
        assert_eq!(once.vertical_speed, twice.vertical_speed);
    }

    /// Circles for two minutes with an airspeed sensor at the given
    /// dilution, and reports how sure the wind estimate is.
    fn wind_uncertainty(dilution: Option<f64>) -> f64 {
        let mut sensors = AirSensors::default();
        for second in 0..120 {
            let time = at(second * 1_000);
            let mut fix = snapshot(None);
            fix.horizontal_dilution = dilution;
            fix.track = Some(Angle::from_degrees(360. * second as f64 / 20.));
            sensors.pressure_altitude(time, PressureAltitude::new(Length::from_meters(1000.)));
            sensors.air_speed(time, Speed::from_kilometers_per_hour(120.));
            sensors.gps(time, &fix);
        }
        assert_some!(assert_some!(sensors.state()).wind)
            .uncertainty
            .as_meters_per_second()
    }

    #[test]
    fn a_reported_dilution_sharpens_the_wind() {
        // The dilution scales the measurement noise of the wind filter,
        // so a receiver that reports a good one is believed sooner. A
        // source that reports none falls back on a pessimistic value.
        let reported = wind_uncertainty(Some(0.8));
        let assumed = wind_uncertainty(None);

        assert!(
            reported < assumed,
            "dilution 0.8 gave {reported:.4} m/s against {assumed:.4} m/s with none"
        );
    }

    #[test]
    fn nothing_is_reported_before_a_second_altitude() {
        let mut sensors = AirSensors::default();
        sensors.pressure_altitude(at(0), PressureAltitude::new(Length::from_meters(1000.)));

        assert_none!(sensors.state());
    }
}
