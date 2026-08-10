use crate::Frequency;
use updraft_units::{Angle, Length};

/// One field for an unplanned landing.
///
/// An outlanding field is not an airfield. It has no designated runway, no
/// identifier, and no services. CUP style 3 supplies the usable direction
/// and the usable dimensions. No OpenAIP dataset supplies outlanding fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outlanding {
    /// The usable landing direction from the CUP `rwdir` column.
    pub direction: Option<Angle>,
    /// The usable length from the CUP `rwlen` column.
    pub length: Option<Length>,
    /// The usable width from the CUP `rwwidth` column.
    pub width: Option<Length>,
    /// The frequency from the CUP `freq` column. The CUP format permits it
    /// for an outlanding field.
    pub frequency: Option<Frequency>,
}
