use crate::{Frequency, OperatingHours};
use updraft_units::{Angle, Length};

/// One radio navigation aid.
///
/// CUP supplies only the kind. OpenAIP supplies the transmission values.
#[derive(Clone, Debug, PartialEq)]
pub struct Navaid {
    pub navaid_type: NavaidType,
    /// The published identifier, for example `FFM`.
    pub identifier: Option<Box<str>>,
    /// The TACAN channel, for example `108X`.
    pub channel: Option<Box<str>>,
    pub frequency: Option<Frequency>,
    /// The published reception range.
    pub range: Option<Length>,
    pub magnetic_declination: Option<Angle>,
    pub aligned_true_north: Option<bool>,
    pub hours_of_operation: Option<OperatingHours>,
}

/// The kind of a radio navigation aid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavaidType {
    /// CUP style 10 and OpenAIP navaid type 2.
    Ndb,
    /// CUP style 9 and OpenAIP navaid type 3.
    Vor,
    /// OpenAIP navaid type 0.
    Dme,
    /// OpenAIP navaid type 1.
    Tacan,
    /// OpenAIP navaid type 4.
    VorDme,
    /// OpenAIP navaid type 5.
    Vortac,
    /// OpenAIP navaid type 6.
    Dvor,
    /// OpenAIP navaid type 7.
    DvorDme,
    /// OpenAIP navaid type 8.
    Dvortac,
}
