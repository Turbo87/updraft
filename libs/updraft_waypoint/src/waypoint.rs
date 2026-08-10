use crate::{
    Airfield, HangGlidingSite, Hotspot, Navaid, Obstacle, Outlanding, RcAirfield, ReportingPoint,
};
use updraft_geo::LatLon;
use updraft_units::MslAltitude;

/// A stable sequence number within one parsed waypoint dataset.
///
/// The number is stable only for that dataset. It is not durable across a
/// source replacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaypointId(pub u32);

/// One canonical point of interest.
#[derive(Clone, Debug, PartialEq)]
pub struct Waypoint {
    /// The stable sequence number within this dataset.
    pub id: WaypointId,
    /// The source name. CUP and OpenAIP both require it.
    pub name: Box<str>,
    /// The short code from the CUP `code` column.
    pub code: Option<Box<str>>,
    /// The unvalidated source country codes.
    ///
    /// CUP uses IANA top-level domains. OpenAIP uses ISO 3166-1 alpha-2
    /// codes and permits more than one. The model keeps the source text and
    /// applies no country registry.
    pub country_codes: Vec<Box<str>>,
    /// The point position.
    pub position: LatLon,
    /// The elevation when the source supplies one.
    ///
    /// OpenAIP always supplies MSL elevation. The CUP column is optional and
    /// declares no reference datum. The importer reads it as MSL.
    pub elevation: Option<MslAltitude>,
    /// The point kind with its kind-specific attributes.
    pub kind: WaypointKind,
    /// The CUP description or the OpenAIP remarks.
    pub description: Option<Box<str>>,
}

/// The kind of a point of interest.
///
/// The variants are the union of the CUP waypoint styles and the OpenAIP
/// point types. A variant carries a payload when its source family supplies
/// attributes that no other family uses.
#[derive(Clone, Debug, PartialEq)]
pub enum WaypointKind {
    /// An unknown kind. CUP style 0 and every unmapped source value.
    Unknown,
    /// A turn point without further classification. CUP style 1.
    Waypoint,
    /// An airfield. CUP styles 2, 4, and 5, and the OpenAIP airports
    /// dataset.
    ///
    /// The payload is boxed, because it is much larger than every other
    /// payload. Most waypoints of a dataset are not airfields.
    Airfield(Box<Airfield>),
    /// A field for an unplanned landing. CUP style 3.
    Outlanding(Outlanding),
    /// CUP style 6.
    MountainPass,
    /// CUP style 7.
    MountainTop,
    /// A vertical obstruction. CUP styles 8 and 11, and the OpenAIP
    /// obstacles dataset.
    Obstacle(Obstacle),
    /// A radio navigation aid. CUP styles 9 and 10, and the OpenAIP navaids
    /// dataset.
    Navaid(Navaid),
    /// CUP style 12.
    Dam,
    /// CUP style 13.
    Tunnel,
    /// CUP style 14.
    Bridge,
    /// CUP style 15.
    PowerPlant,
    /// CUP style 16.
    Castle,
    /// CUP style 17.
    Intersection,
    /// CUP style 18.
    Marker,
    /// CUP style 19 and the OpenAIP reporting points dataset.
    ReportingPoint(ReportingPoint),
    /// The OpenAIP hotspots dataset.
    Hotspot(Hotspot),
    /// CUP styles 20 and 21, and the OpenAIP hang gliding dataset.
    HangGlidingSite(HangGlidingSite),
    /// The OpenAIP RC airfields dataset.
    RcAirfield(RcAirfield),
}

/// One parsed waypoint source.
#[derive(Clone, Debug, PartialEq)]
pub struct WaypointDataset {
    waypoints: Vec<Waypoint>,
}

impl WaypointDataset {
    /// Creates a canonical dataset from waypoints in parser order.
    pub fn from_waypoints(waypoints: Vec<Waypoint>) -> Self {
        Self { waypoints }
    }

    /// Returns every waypoint in parser order.
    pub fn waypoints(&self) -> &[Waypoint] {
        &self.waypoints
    }
}
