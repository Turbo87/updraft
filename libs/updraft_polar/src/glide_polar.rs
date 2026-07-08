use updraft_units::{Mass, Speed};

use crate::PolarCoefficients;

/// A glider's polar, adjusted for the current wing loading and bug
/// contamination.
///
/// The polar is defined by [`PolarCoefficients`] measured at a *reference
/// mass*. Two adjustments map it to the configured glider:
///
/// - **Mass** (crew, equipment, water ballast): a heavier glider flies
///   the same polar stretched along both axes by `√(m/m_ref)`, sinking
///   faster but at higher speeds with an unchanged best glide ratio.
/// - **Bugs**: contamination degrades performance across the board. A
///   bugs value between 0% and 100% is the fraction of clean performance lost.
///
/// Sink rates returned by this type are positive numbers. Inputs like
/// the MacCready setting and netto instead follow the convention, where
/// positive means climbing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlidePolar {
    ideal: PolarCoefficients,
    reference_mass: Mass,
    total_mass: Mass,
    bugs: f64,
    adjusted: PolarCoefficients,
}

impl GlidePolar {
    /// Creates a polar from coefficients measured at `reference_mass`,
    /// with the total mass initially equal to the reference mass and a
    /// clean wing. Returns `None` unless the reference mass is finite
    /// and positive.
    pub fn new(ideal: PolarCoefficients, reference_mass: Mass) -> Option<Self> {
        let valid = reference_mass.as_kilograms().is_finite() && reference_mass > Mass::ZERO;
        valid.then_some(Self {
            ideal,
            reference_mass,
            total_mass: reference_mass,
            bugs: 0.,
            adjusted: ideal,
        })
    }

    /// Sets the total flying mass (empty mass + crew + water ballast).
    /// Non-finite or non-positive values leave the mass unchanged.
    pub fn with_total_mass(mut self, total_mass: Mass) -> Self {
        if total_mass.as_kilograms().is_finite() && total_mass > Mass::ZERO {
            self.total_mass = total_mass;
            self.update();
        }
        self
    }

    /// Sets the bugs value: the fraction of clean performance lost,
    /// valid in `0.0..1.0`. Values outside that range leave the value
    /// unchanged.
    pub fn with_bugs(mut self, bugs: f64) -> Self {
        if (0. ..1.).contains(&bugs) {
            self.bugs = bugs;
            self.update();
        }
        self
    }

    /// Recomputes the adjusted coefficients after a parameter change.
    ///
    /// Scaling the ideal polar `w(v)` to `w'(v) = k·w(v/k) / (1 - bugs)`
    /// with the loading factor `k = √(m/m_ref)` maps the coefficients to
    /// `a/k`, `b`, `c·k`, all divided by the remaining performance
    /// fraction `1 - bugs`.
    fn update(&mut self) {
        let loading_factor = (self.total_mass / self.reference_mass).sqrt();
        let degradation = 1. / (1. - self.bugs);
        self.adjusted = PolarCoefficients::new(
            self.ideal.a() * degradation / loading_factor,
            self.ideal.b() * degradation,
            self.ideal.c() * degradation * loading_factor,
        )
        .expect("mass and bugs adjustments preserve polar validity");
    }

    /// The coefficients adjusted for the current mass and bugs settings.
    pub fn coefficients(&self) -> PolarCoefficients {
        self.adjusted
    }

    /// The unadjusted coefficients at reference mass with a clean wing.
    pub fn ideal_coefficients(&self) -> PolarCoefficients {
        self.ideal
    }

    /// The mass at which the ideal coefficients were measured.
    pub fn reference_mass(&self) -> Mass {
        self.reference_mass
    }

    /// The configured total flying mass.
    pub fn total_mass(&self) -> Mass {
        self.total_mass
    }

    /// The configured bugs value (`0.0` = clean wing).
    pub fn bugs(&self) -> f64 {
        self.bugs
    }

    /// The still-air sink rate (a positive number) at the given airspeed.
    pub fn sink_rate(&self, air_speed: Speed) -> Speed {
        self.adjusted.sink_rate(air_speed)
    }

    /// The glide ratio at the given airspeed in still air.
    pub fn glide_ratio(&self, air_speed: Speed) -> f64 {
        air_speed / self.sink_rate(air_speed)
    }

    /// The airspeed of minimum sink.
    pub fn min_sink_speed(&self) -> Speed {
        self.adjusted.min_sink_speed()
    }

    /// The sink rate (a positive number) at minimum sink speed.
    pub fn min_sink_rate(&self) -> Speed {
        self.adjusted.min_sink_rate()
    }

    /// The airspeed of best glide in still air.
    pub fn best_glide_speed(&self) -> Speed {
        self.adjusted.best_glide_speed()
    }

    /// The best (maximum) glide ratio in still air.
    pub fn best_glide_ratio(&self) -> f64 {
        self.glide_ratio(self.best_glide_speed())
    }

    /// The MacCready speed to fly through air moving vertically at
    /// `netto` (positive up) with the MacCready ring set to `mac_cready`,
    /// against a wind component `headwind` (negative for a tailwind).
    ///
    /// Maximizing the achieved speed over ground,
    /// `d/dv [(mc + w(v) - netto) / (v - v_wind)] = 0`, solves to
    /// `v = v_wind + √(v_wind² + (b·v_wind + c + mc - netto) / a)` for
    /// the quadratic polar. With zero wind this is the classic MacCready
    /// ring relation `v = √((c + mc - netto) / a)`.
    ///
    /// The result is clamped to no less than the minimum sink speed: in
    /// air rising more strongly than the ring setting the theory would
    /// command ever slower flight.
    pub fn speed_to_fly(&self, mac_cready: Speed, netto: Speed, headwind: Speed) -> Speed {
        let excess = self.adjusted.c() + (mac_cready - netto).as_meters_per_second();
        let wind = headwind.as_meters_per_second();
        let discriminant = wind * wind + (self.adjusted.b() * wind + excess) / self.adjusted.a();
        let min_sink_speed = self.min_sink_speed();
        if discriminant <= 0. {
            return min_sink_speed;
        }

        let speed = Speed::from_meters_per_second(wind + discriminant.sqrt());
        if speed > min_sink_speed {
            speed
        } else {
            min_sink_speed
        }
    }

    /// The classic MacCready cross-country speed: the average speed
    /// achieved alternating climbs at `mac_cready` with cruise at the
    /// corresponding speed to fly, in still air:
    /// `v · mc / (mc + w(v))` at `v = speed_to_fly(mc, 0, 0)`.
    ///
    /// Returns zero for a non-positive MacCready value: with no climb
    /// available, the theoretical average degenerates to zero.
    pub fn cross_country_speed(&self, mac_cready: Speed) -> Speed {
        let mc = mac_cready.as_meters_per_second();
        if mc <= 0. {
            return Speed::ZERO;
        }

        let speed = self.speed_to_fly(mac_cready, Speed::ZERO, Speed::ZERO);
        let sink = self.sink_rate(speed).as_meters_per_second();
        speed * (mc / (mc + sink))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    fn kmh(value: f64) -> Speed {
        Speed::from_kilometers_per_hour(value)
    }

    fn mps(value: f64) -> Speed {
        Speed::from_meters_per_second(value)
    }

    /// LS8 15m: 360 kg reference mass, best glide ~44 at ~112 km/h.
    fn ls8() -> GlidePolar {
        let coefficients = PolarCoefficients::from_points([
            (kmh(100.), mps(0.67)),
            (kmh(155.), mps(1.45)),
            (kmh(185.), mps(2.5)),
        ])
        .unwrap();
        GlidePolar::new(coefficients, Mass::from_kilograms(360.)).unwrap()
    }

    #[test]
    fn defaults_to_reference_mass_and_clean_wing() {
        let polar = ls8();
        assert_eq!(polar.total_mass(), polar.reference_mass());
        assert_eq!(polar.bugs(), 0.);
        assert_eq!(polar.coefficients(), polar.ideal_coefficients());
        assert_abs_diff_eq!(polar.best_glide_ratio(), 43.6, epsilon = 0.1);
        assert_abs_diff_eq!(polar.best_glide_speed(), kmh(111.6), epsilon = 0.1);
    }

    #[test]
    fn rejects_invalid_reference_mass() {
        let coefficients = ls8().ideal_coefficients();
        assert!(GlidePolar::new(coefficients, Mass::ZERO).is_none());
        assert!(GlidePolar::new(coefficients, Mass::from_kilograms(-1.)).is_none());
        assert!(GlidePolar::new(coefficients, Mass::from_kilograms(f64::NAN)).is_none());
    }

    #[test]
    fn ballast_stretches_the_polar() {
        let dry = ls8();
        // 21% more mass -> loading factor 1.1.
        let ballasted = dry.with_total_mass(Mass::from_kilograms(360. * 1.21));

        // Speeds scale by 1.1, sink rates by 1.1, the ratio is roughly unchanged.
        assert_abs_diff_eq!(ballasted.min_sink_speed() / dry.min_sink_speed(), 1.1);
        assert_abs_diff_eq!(ballasted.best_glide_speed() / dry.best_glide_speed(), 1.1);
        assert_abs_diff_eq!(ballasted.min_sink_rate() / dry.min_sink_rate(), 1.1);
        assert_abs_diff_eq!(
            ballasted.best_glide_ratio(),
            dry.best_glide_ratio(),
            epsilon = 1e-13
        );

        // At a fixed (high) speed the heavier glider sinks *less*.
        assert!(ballasted.sink_rate(kmh(180.)) < dry.sink_rate(kmh(180.)));

        // Invalid masses are ignored.
        assert_eq!(ballasted.with_total_mass(Mass::ZERO), ballasted);
        assert_eq!(
            ballasted.with_total_mass(Mass::from_kilograms(f64::NAN)),
            ballasted
        );
    }

    #[test]
    fn bugs_degrade_performance() {
        let clean = ls8();
        let dirty = clean.with_bugs(0.2);

        // Sink rates scale by 1/0.8 at every speed, ratios by 0.8, and
        // the characteristic speeds stay put.
        let sink_rate_ratio = dirty.sink_rate(kmh(120.)) / clean.sink_rate(kmh(120.));
        assert_abs_diff_eq!(sink_rate_ratio, 1.25, epsilon = 0.01);
        assert_abs_diff_eq!(dirty.min_sink_speed() / clean.min_sink_speed(), 1.);
        assert_abs_diff_eq!(dirty.best_glide_speed() / clean.best_glide_speed(), 1.);
        assert_abs_diff_eq!(
            dirty.best_glide_ratio(),
            0.8 * clean.best_glide_ratio(),
            epsilon = 1e-12
        );

        // Values outside 0.0..1.0 are ignored.
        assert_eq!(clean.with_bugs(-0.5), clean);
        assert_eq!(clean.with_bugs(1.), clean);
        assert_eq!(clean.with_bugs(f64::NAN), clean);
    }

    #[test]
    fn speed_to_fly() {
        let polar = ls8();

        // MacCready zero in still air commands best glide speed.
        let mc0 = polar.speed_to_fly(Speed::ZERO, Speed::ZERO, Speed::ZERO);
        assert_abs_diff_eq!(mc0, kmh(111.6), epsilon = 0.1);
        assert_abs_diff_eq!(mc0, polar.best_glide_speed());

        // Higher ring settings command higher speeds (LS8 at MC 1.0: ~129 km/h).
        let mc1 = polar.speed_to_fly(mps(1.), Speed::ZERO, Speed::ZERO);
        assert_abs_diff_eq!(mc1, kmh(128.6), epsilon = 0.1);

        let mc2 = polar.speed_to_fly(mps(2.), Speed::ZERO, Speed::ZERO);
        assert_abs_diff_eq!(mc2, kmh(143.6), epsilon = 0.1);

        // Sinking air commands higher speeds, rising air lower speeds.
        let mc1_sink = polar.speed_to_fly(mps(1.), mps(-2.), Speed::ZERO);
        assert_abs_diff_eq!(mc1_sink, kmh(157.1), epsilon = 0.1);
        let mc1_lift = polar.speed_to_fly(mps(1.), mps(0.5), Speed::ZERO);
        assert_abs_diff_eq!(mc1_lift, kmh(120.4), epsilon = 0.1);

        // Air rising more strongly than the ring setting clamps to
        // minimum sink speed instead of commanding ever slower flight.
        let mc1_strong_lift = polar.speed_to_fly(mps(1.), mps(5.), Speed::ZERO);
        assert_abs_diff_eq!(mc1_strong_lift, kmh(98.5), epsilon = 0.1);

        // Moderate lift at MC 0 commands a speed below minimum sink, which also clamps.
        let mc0_lift = polar.speed_to_fly(Speed::ZERO, mps(2.), Speed::ZERO);
        assert_abs_diff_eq!(mc0_lift, kmh(98.5), epsilon = 0.1);

        // Ballast shifts the glide polar to higher speeds.
        let ballasted = polar.with_total_mass(Mass::from_kilograms(490.));
        let mc1_ballasted = ballasted.speed_to_fly(mps(1.), Speed::ZERO, Speed::ZERO);
        assert_abs_diff_eq!(mc1_ballasted, kmh(147.3), epsilon = 0.1);
    }

    #[test]
    fn speed_to_fly_in_wind() {
        let polar = ls8();

        // A headwind commands a higher speed, a tailwind a lower one.
        let headwind = polar.speed_to_fly(mps(1.), Speed::ZERO, kmh(30.));
        assert_abs_diff_eq!(headwind, kmh(137.3), epsilon = 0.1);
        let tailwind = polar.speed_to_fly(mps(1.), Speed::ZERO, kmh(-30.));
        assert_abs_diff_eq!(tailwind, kmh(122.8), epsilon = 0.1);

        // Strong lift still clamps to minimum sink speed.
        let tailwind_lift = polar.speed_to_fly(mps(1.), mps(5.), kmh(-30.));
        assert_abs_diff_eq!(tailwind_lift, kmh(98.5), epsilon = 0.1);

        // In moderate lift a headwind can push the discriminant negative,
        // which clamps as well.
        let headwind_lift = polar.speed_to_fly(Speed::ZERO, mps(2.), kmh(30.));
        assert_abs_diff_eq!(headwind_lift, kmh(98.5), epsilon = 0.1);

        // Ever stronger tailwinds approach minimum sink speed from above.
        let strong_tailwind = polar.speed_to_fly(mps(1.), Speed::ZERO, kmh(-100.));
        assert_abs_diff_eq!(strong_tailwind, kmh(115.), epsilon = 0.1);
    }

    #[test]
    fn cross_country_speed() {
        let polar = ls8();

        assert_eq!(polar.cross_country_speed(Speed::ZERO), Speed::ZERO);
        assert_eq!(polar.cross_country_speed(mps(-1.)), Speed::ZERO);

        // LS8 at MC 1.0 achieves ~68 km/h on the classic MacCready ring.
        let mc1 = polar.cross_country_speed(mps(1.));
        assert_abs_diff_eq!(mc1, kmh(68.), epsilon = 0.1);

        // Stronger climbs mean faster progress, but never faster than
        // the cruise itself.
        let mc2 = polar.cross_country_speed(mps(2.));
        assert_abs_diff_eq!(mc2, kmh(90.7), epsilon = 0.1);
    }
}
