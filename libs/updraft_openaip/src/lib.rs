//! Deserialization of the OpenAIP JSON datasets.
//!
//! OpenAIP publishes one daily export for each country and dataset type. The
//! files are public objects of an S3 bucket:
//!
//! ```text
//! https://s3.openaip.net/openaip-system-exports/<country>_<dataset>.<format>
//! ```
//!
//! `country` is a lowercase ISO 3166-1 alpha-2 code, `dataset` is one of the
//! suffixes below, and `format` is `json` for the files that this crate reads.
//! A country that has no data for a dataset has no file. The export overview at
//! <https://www.openaip.net/data/exports> lists the completed export jobs.
//!
//! | Suffix | Module              | Type                                |
//! |--------|---------------------|-------------------------------------|
//! | `asp`  | [`airspace`]        | [`airspace::Airspace`]              |
//! | `apt`  | [`airport`]         | [`airport::Airport`]                |
//! | `nav`  | [`navaid`]          | [`navaid::Navaid`]                  |
//! | `rpp`  | [`reporting_point`] | [`reporting_point::ReportingPoint`] |
//! | `hot`  | [`hotspot`]         | [`hotspot::Hotspot`]                |
//! | `hgl`  | [`hang_gliding`]    | [`hang_gliding::HangGlidingSite`]   |
//! | `obs`  | [`obstacle`]        | [`obstacle::Obstacle`]              |
//!
//! Each file holds a JSON array of records. The crate supplies the record types
//! and leaves the JSON reader to the caller:
//!
//! ```no_run
//! use updraft_openaip::airspace::Airspace;
//!
//! let bytes = std::fs::read("de_asp.json")?;
//! let airspaces: Vec<Airspace> = serde_json::from_slice(&bytes)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The records keep the OpenAIP model. They do not use the Updraft canonical
//! types, and they do not convert units. The GeoJSON export carries the same
//! records with the geometry moved into the feature and every other field moved
//! into the feature properties.
//!
//! # Model rules
//!
//! The exports are more permissive than the published response schemas at
//! <https://docs.openaip.net>. The types therefore follow these rules:
//!
//! - A numeric classification becomes an enum. An unsupported code becomes
//!   `Unsupported` and keeps its number. OpenAIP adds codes without a model
//!   version change, so a new code must not reject a complete dataset.
//! - An unknown field is ignored. The exports already carry fields that the
//!   response schemas do not document.
//! - A field is required only when it is present in every record of the sampled
//!   datasets. The response schemas mark no field as required, including fields
//!   such as `_id` and `geometry`.
//! - An absent array becomes an empty list.
//! - A timestamp stays an RFC 3339 string, and a time of day stays an `HH:MM`
//!   string.

mod code;
mod common;

pub mod airport;
pub mod airspace;
pub mod hang_gliding;
pub mod hotspot;
pub mod navaid;
pub mod obstacle;
pub mod reporting_point;

pub use common::{
    Countries, DayOfWeek, Elevation, ElevationGeoid, FrequencyUnit, HoursOfOperation, Image,
    OperatingHours, Point, Polygon, Position, Ring, VerticalDatum, VerticalLimit, VerticalUnit,
    WindDirection,
};
