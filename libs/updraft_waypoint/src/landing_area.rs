use updraft_units::{Angle, Length, Mass};

/// One usable landing area.
///
/// A runway and an outlanding field share this shape. Both have a usable
/// direction, usable dimensions, and a surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LandingArea {
    /// The usable landing direction. The CUP column declares no reference
    /// datum. OpenAIP supplies a true heading.
    pub direction: Option<Angle>,
    pub length: Option<Length>,
    pub width: Option<Length>,
    pub surface: Option<Surface>,
}

/// The surface of a landing area.
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    pub compositions: Vec<SurfaceComposition>,
    pub main_composition: Option<SurfaceComposition>,
    pub condition: Option<SurfaceCondition>,
    /// The maximum take-off weight that the surface permits.
    pub max_take_off_weight: Option<Mass>,
    /// The unvalidated pavement classification number.
    pub pcn: Option<Box<str>>,
    pub remarks: Option<Box<str>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceComposition {
    Asphalt,
    Concrete,
    Grass,
    Sand,
    Water,
    BituminousTar,
    Brick,
    Macadam,
    Stone,
    Coral,
    Clay,
    Laterite,
    Gravel,
    Earth,
    Ice,
    Snow,
    ProtectiveLaminate,
    Metal,
    LandingMat,
    PiercedSteelPlanking,
    Wood,
    NonBituminousMix,
    Unknown,
    /// A solid surface without further classification. CUP style 5 supplies
    /// it, because CUP names no material.
    Solid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceCondition {
    Good,
    Fair,
    Poor,
    Unsafe,
    Deformed,
    Unknown,
}
