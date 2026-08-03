use crate::time::Timestamp;
use crate::topic::{Instruments, LatLon};
use updraft_geo::LatLon as GeoLatLon;
use updraft_units::{Angle, MslAltitude, Speed};

/// A source value and the monotonic time when the core ingested it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timed<T> {
    pub value: T,
    pub ingested_at: Timestamp,
}

impl<T> Timed<T> {
    /// Uses the supplied monotonic ingestion time without reading a clock.
    pub fn new(value: T, ingested_at: Timestamp) -> Self {
        Self { value, ingested_at }
    }
}

/// Stores the latest GPS values from one source with independent ingestion times.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpsCandidate {
    pub position: Option<Timed<GeoLatLon>>,
    pub altitude: Option<Timed<MslAltitude>>,
    pub track: Option<Timed<Angle>>,
    pub ground_speed: Option<Timed<Speed>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OwnshipState {
    pub position: Option<GeoLatLon>,
    pub altitude_msl: Option<MslAltitude>,
    pub track: Option<Angle>,
    pub ground_speed: Option<Speed>,
}

impl OwnshipState {
    pub fn published(self) -> Instruments {
        Instruments {
            position: self.position.map(|position| LatLon {
                latitude_degrees: position.latitude().as_degrees(),
                longitude_degrees: position.longitude().as_degrees(),
            }),
            track_degrees: self.track.map(Angle::as_degrees),
            ground_speed_meters_per_second: self.ground_speed.map(Speed::as_meters_per_second),
            altitude_msl_meters: self
                .altitude_msl
                .map(|altitude| altitude.into_inner().as_meters()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topic::LatLon;
    use claims::assert_some_eq;
    use updraft_units::Length;

    #[test]
    fn projects_domain_values_to_the_instruments_topic() {
        let ownship = OwnshipState {
            position: Some(GeoLatLon::from_degrees(50.823, 6.186)),
            altitude_msl: Some(MslAltitude::new(Length::from_meters(200.0))),
            track: Some(Angle::from_degrees(270.0)),
            ground_speed: Some(Speed::from_meters_per_second(45.0)),
        };

        let published = ownship.published();

        assert_some_eq!(published.altitude_msl_meters, 200.0);
        assert_some_eq!(published.track_degrees, 270.0);
        assert_some_eq!(published.ground_speed_meters_per_second, 45.0);
        assert_some_eq!(
            published.position,
            LatLon {
                latitude_degrees: 50.823,
                longitude_degrees: 6.186,
            }
        );
    }
}
