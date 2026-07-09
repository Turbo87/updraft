use updraft_polar::{GlidePolar, PolarCoefficients};
use updraft_units::{Area, Length, Mass, Speed};

/// A read-only catalogue entry describing an aircraft type.
///
/// A preset is a three-level tree: a *base model*, its *build/propulsion
/// [variants](Variant)*, and each variant's *[wingspan
/// configurations](WingspanConfig)*. A user aircraft profile copies a
/// single variant, pinning one airframe (build variant and propulsion
/// fixed) while possibly still carrying multiple wingspan configurations
/// (e.g. removable tips).
///
/// Most fields throughout the tree are optional: the catalogue is
/// deliberately allowed to be incomplete, and a profile created from a
/// preset can override or fill in any field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AircraftPreset {
    /// Base model name, e.g. `"ASG 29"`.
    pub name: &'static str,
    /// Manufacturer, where known, e.g. `"Alexander Schleicher"`.
    pub manufacturer: Option<&'static str>,
    /// Number of seats. A property of the base model: the seat count does
    /// not vary between build/propulsion variants (a two-seat airframe
    /// like the ASG 32 is a different base model from any single-seater).
    pub seats: u8,
    /// Build/propulsion variants of this base model. Always at least one.
    pub variants: &'static [Variant],
}

/// A build/propulsion variant of a base model, e.g. `"ASG 29E"` of the
/// base model `"ASG 29"`.
///
/// A variant fixes the airframe (build variant and propulsion) but may
/// still offer several [wingspan configurations](WingspanConfig).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Variant {
    /// Display name of the variant, e.g. `"ASG 29E"`.
    pub name: &'static str,
    /// How the variant is propelled, where known.
    pub propulsion: Option<Propulsion>,
    /// Wingspan configurations (the leaves of the tree). Always at least
    /// one; a fixed-span glider has exactly one and should not be shown a
    /// span picker.
    pub wingspans: &'static [WingspanConfig],
}

/// How a variant is propelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Propulsion {
    /// Pure glider, no engine.
    Pure,
    /// Self-launching motor glider (retractable engine or jet).
    SelfLaunch,
    /// Sustainer / turbo: an engine that extends the glide but cannot
    /// self-launch.
    Sustainer,
    /// Front-electric sustainer (FES).
    ElectricSustainer,
}

/// A single wingspan configuration: the leaf of the preset tree.
///
/// The leaf carries most of the numeric data because these values *vary
/// by wingspan* (e.g. a glider with removable tips has different
/// coefficients, areas, and masses at 15 m and 18 m).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WingspanConfig {
    /// Wingspan, where known.
    pub wingspan: Option<Length>,
    /// Wing area, where known.
    pub wing_area: Option<Area>,
    /// Glide polar and its reference mass, where known.
    pub polar: Option<Polar>,
    /// Empty mass: the bare airframe per the weighing report. An
    /// informational value and a weight-and-balance input, but *not* a
    /// polar input.
    pub empty_mass: Option<Mass>,
    /// Maximum take-off mass.
    ///
    /// This is a **limit for overload warnings only**, never a
    /// computation input. It is deliberately kept separate from the
    /// [reference mass](Polar::reference_mass) and the total flying mass
    /// so it is never mistaken for a physics value.
    pub max_takeoff_mass: Option<Mass>,
    /// Water ballast tank capacity, expressed as the mass of water the
    /// tanks hold when full, where known.
    pub water_ballast_capacity: Option<Mass>,
    /// Never-exceed speed (V<sub>NE</sub>), where known.
    pub vne: Option<Speed>,
    /// Stall speed, where known.
    pub stall_speed: Option<Speed>,
    /// Flap speed ranges, where known. Empty for fixed-wing gliders and
    /// wherever the data is not available.
    pub flap_speeds: &'static [FlapSpeedRange],
    /// Centre-of-gravity limits, where known.
    pub cg_limits: Option<CgLimits>,
    /// The aircraft's ID in the `WeGlide` aircraft database, where known.
    pub weglide_id: Option<u32>,
    /// Fallback handicap (`DMSt` / `DAeC` index), used offline.
    ///
    /// Handicaps change over time, so a value baked into the catalogue
    /// bit-rots. The live value is fetched from the `WeGlide` API when
    /// online; this is the build-time fallback for offline use and may be
    /// stale.
    pub fallback_handicap: Option<f64>,
}

/// A flap setting and the airspeed range over which it is used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlapSpeedRange {
    /// The flap setting label, e.g. `"+2"`, `"L"`, `"0"`.
    pub setting: &'static str,
    /// Lowest airspeed for this setting, where known.
    pub min_speed: Option<Speed>,
    /// Highest airspeed for this setting, where known.
    pub max_speed: Option<Speed>,
}

/// Centre-of-gravity limits, as arm distances from the datum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CgLimits {
    /// Forward CG limit (arm from the datum).
    pub forward: Length,
    /// Aft CG limit (arm from the datum).
    pub aft: Length,
}

/// A glide polar in the catalogue: the reduced quadratic coefficients
/// plus the mass they are valid at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Polar {
    /// Quadratic sink coefficients in reduced form: sink in m/s (positive
    /// down) at an airspeed given in multiples of 100 km/h, valid at the
    /// reference mass with a clean wing.
    coefficients: (f64, f64, f64),
    /// The mass the coefficients were measured at (glider plus standard
    /// pilot, no ballast). Feeds the polar math.
    pub reference_mass: Mass,
}

impl Polar {
    /// Builds a polar from reduced coefficients and a reference mass.
    ///
    /// The coefficients are in the reduced form used by the source
    /// catalogues (sink in m/s at an airspeed in multiples of 100 km/h),
    /// converted to SI on demand by [`coefficients`](Self::coefficients).
    pub const fn new(coefficients: (f64, f64, f64), reference_mass: Mass) -> Self {
        Self {
            coefficients,
            reference_mass,
        }
    }

    /// This polar's coefficients converted to the SI form used by
    /// [`PolarCoefficients`].
    pub fn coefficients(&self) -> PolarCoefficients {
        /// The conversion factor from the reduced (LX-like) coefficients
        /// to SI.
        const CONVERSION_FACTOR: f64 = 100. / 3.6;

        let (a, b, c) = self.coefficients;
        let a = a / (CONVERSION_FACTOR * CONVERSION_FACTOR);
        let b = b / CONVERSION_FACTOR;
        PolarCoefficients::new(a, b, c).expect("built-in polar coefficients are valid")
    }

    /// The glide polar at the reference mass with a clean wing.
    pub fn glide_polar(&self) -> GlidePolar {
        GlidePolar::new(self.coefficients(), self.reference_mass)
            .expect("built-in polar reference mass is valid")
    }
}

impl AircraftPreset {
    /// Iterates over every [wingspan configuration](WingspanConfig) of
    /// every [variant](Variant) of this preset.
    pub fn wingspans(&self) -> impl Iterator<Item = &'static WingspanConfig> {
        self.variants
            .iter()
            .flat_map(|variant| variant.wingspans.iter())
    }
}
