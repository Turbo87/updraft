use crate::{Frequency, LandingArea};

/// One field for an unplanned landing.
///
/// An outlanding field is not an airfield. It has no designated runway, no
/// identifier, and no services. CUP style 3 supplies the usable area. No
/// OpenAIP dataset supplies outlanding fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Outlanding {
    /// The direction, the dimensions, and the surface.
    pub area: LandingArea,
    /// The frequency from the CUP `freq` column. The CUP format permits it
    /// for an outlanding field.
    pub frequency: Option<Frequency>,
}
