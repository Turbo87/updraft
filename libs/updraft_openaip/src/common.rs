//! Model elements that more than one dataset uses.

use crate::code::codes;
use serde::{Deserialize, Deserializer};

/// A `WGS84` position as `[longitude, latitude]` in decimal degrees.
pub type Position = [f64; 2];

/// One closed ring of a polygon.
pub type Ring = Vec<Position>;

/// A `GeoJSON` point geometry.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Point {
    pub coordinates: Position,
}

/// A `GeoJSON` polygon geometry.
///
/// The published schema permits one ring. Some records carry more. The United
/// States dataset uses additional rings for disjoint areas instead of holes.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Polygon {
    pub coordinates: Vec<Ring>,
}

/// Source country codes in dataset order.
///
/// OpenAIP writes one ISO 3166-1 alpha-2 code or an array of codes. Both forms
/// deserialize into this list. The codes stay unvalidated source text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Countries(pub Vec<Box<str>>);

impl<'de> Deserialize<'de> for Countries {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OneOrMany {
            One(Box<str>),
            Many(Vec<Box<str>>),
        }

        Ok(match OneOrMany::deserialize(deserializer)? {
            OneOrMany::One(code) => Self(vec![code]),
            OneOrMany::Many(codes) => Self(codes),
        })
    }
}

codes! {
    /// The unit of a vertical value.
    pub enum VerticalUnit {
        0 => Meter,
        1 => Foot,
        6 => FlightLevel,
    }
}

codes! {
    /// The reference datum of a vertical value.
    pub enum VerticalDatum {
        /// Ground level.
        0 => Gnd,
        /// Mean sea level.
        1 => Msl,
        /// Standard pressure altitude.
        2 => Std,
    }
}

/// A vertical airspace limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerticalLimit {
    pub value: i32,
    pub unit: VerticalUnit,
    pub reference_datum: VerticalDatum,
}

/// An elevation above mean sea level.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Elevation {
    pub value: f64,
    pub unit: VerticalUnit,
    pub reference_datum: VerticalDatum,
}

/// The geoid values behind an elevation.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevationGeoid {
    /// Height above the `WGS84` ellipsoid in metres.
    pub hae: f64,
    /// Geoid height in metres.
    pub geoid_height: f64,
}

codes! {
    /// The unit of a radio frequency value.
    pub enum FrequencyUnit {
        1 => KiloHertz,
        2 => MegaHertz,
    }
}

codes! {
    /// A day of the week.
    pub enum DayOfWeek {
        0 => Monday,
        1 => Tuesday,
        2 => Wednesday,
        3 => Thursday,
        4 => Friday,
        5 => Saturday,
        6 => Sunday,
    }
}

/// The hours of operation of an object.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoursOfOperation {
    /// One entry for each day of the week. Some records carry only remarks and
    /// no entries.
    #[serde(default)]
    pub operating_hours: Vec<OperatingHours>,
    pub remarks: Option<Box<str>>,
}

/// The operating hours of one day.
///
/// `start_time` and `end_time` are absent when `sunrise` or `sunset` replaces
/// them. Both use `HH:MM` or `HH:MM:SS` local time.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatingHours {
    pub day_of_week: DayOfWeek,
    pub start_time: Option<Box<str>>,
    pub end_time: Option<Box<str>>,
    /// Operation starts at sunrise.
    pub sunrise: bool,
    /// Operation ends at sunset.
    pub sunset: bool,
    /// Operation depends on a NOTAM.
    pub by_notam: bool,
    pub public_holidays_excluded: bool,
    pub remarks: Option<Box<str>>,
}

/// An image that OpenAIP hosts for an object.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    #[serde(rename = "_id")]
    pub id: Box<str>,
    pub filename: Box<str>,
    pub description: Option<Box<str>>,
}

codes! {
    /// A compass wind direction.
    pub enum WindDirection {
        0 => North,
        1 => NorthEast,
        2 => East,
        3 => SouthEast,
        4 => South,
        5 => SouthWest,
        6 => West,
        7 => NorthWest,
    }
}
