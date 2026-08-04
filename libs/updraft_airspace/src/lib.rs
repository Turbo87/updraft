//! Canonical airspace data.

mod openair;
mod types;

pub use types::{
    Airspace, AirspaceActivity, AirspaceAltitude, AirspaceClass, AirspaceDataset,
    AirspaceGeometryError, AirspaceId, AirspaceImportError, AirspaceParseError, AirspaceType,
};
