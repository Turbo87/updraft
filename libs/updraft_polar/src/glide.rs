use crate::{GlidePolar, isa_density_ratio};
use updraft_units::{Length, Speed};

/// A straight glide through uniform wind and still vertical air.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlideSolution {
    pub true_air_speed: Speed,
    pub ground_speed: Speed,
    pub height_loss: Length,
}

impl GlidePolar {
    /// Selects TAS to minimise `(sink + MC) / ground_speed` along the target track.
    /// Height loss uses physical sink only. Tailwind is positive towards the target.
    /// Crosswind can have either sign. Altitude sets a constant ISA density for the glide.
    ///
    /// The search repeatedly halves the speed range. At each midpoint, the slope
    /// tells us whether flying faster or slower improves the result. Twelve steps
    /// put the selected speed within 0.05 km/h of the optimum. This keeps the
    /// calculation fast without a headwind-based estimate or fallback search.
    /// Crosswind makes a direct formula more complex than this bounded search.
    ///
    /// The search runs from density-corrected minimum-sink speed to 400 km/h TAS.
    /// Returns `None` for invalid inputs or when no speed permits positive progress.
    /// Inputs must be finite, distance and MC nonnegative, and ISA density positive.
    pub fn solve_glide(
        &self,
        distance: Length,
        altitude: Length,
        mac_cready: Speed,
        tailwind: Speed,
        crosswind: Speed,
    ) -> Option<GlideSolution> {
        let distance = distance.as_meters();
        let mc = mac_cready.as_meters_per_second();
        let tailwind = tailwind.as_meters_per_second();
        let crosswind = crosswind.as_meters_per_second().abs();
        let inputs = [distance, altitude.as_meters(), mc, tailwind, crosswind];
        if !inputs.iter().all(|value| value.is_finite()) || distance < 0. || mc < 0. {
            return None;
        }
        let root_density = isa_density_ratio(altitude).sqrt();
        if !root_density.is_finite() || root_density <= 0. {
            return None;
        }
        let limit = Speed::from_kilometers_per_hour(400.).as_meters_per_second();
        let min_speed = self.min_sink_speed().as_meters_per_second() / root_density;
        if min_speed > limit || crosswind > limit {
            return None;
        }
        let forward_speed = |speed: f64| ((speed - crosswind) * (speed + crosswind)).sqrt();
        if forward_speed(limit) + tailwind <= 0. {
            return None;
        }
        let coefficients = self.coefficients();
        let a = coefficients.a() * root_density;
        let b = coefficients.b();
        let c = coefficients.c() / root_density;
        let derivative_sign = |speed: f64| {
            let forward = forward_speed(speed);
            let ground = forward + tailwind;
            if ground <= 0. {
                return f64::NEG_INFINITY;
            }
            let sink = (a * speed + b) * speed + c;
            (2. * a * speed + b) * ground * forward - (sink + mc) * speed
        };
        let mut lower = min_speed.max(crosswind);
        let mut upper = limit;
        let selected = if derivative_sign(upper) <= 0. {
            upper
        } else if derivative_sign(lower) >= 0. {
            lower
        } else {
            // Convex sink and concave ground speed give at most one derivative zero on this interval.
            for _ in 0..12 {
                let middle = (lower + upper) / 2.;
                if derivative_sign(middle) > 0. {
                    upper = middle;
                } else {
                    lower = middle;
                }
            }
            (lower + upper) / 2.
        };
        let true_air_speed = Speed::from_meters_per_second(selected);
        let ground_speed = forward_speed(selected) + tailwind;
        let sink = self.sink_rate_at_altitude(altitude, true_air_speed, 1.);
        let height_loss = distance * (sink.as_meters_per_second() / ground_speed);
        if ground_speed <= 0. || !height_loss.is_finite() {
            return None;
        }
        Some(GlideSolution {
            true_air_speed,
            ground_speed: Speed::from_meters_per_second(ground_speed),
            height_loss: Length::from_meters(height_loss),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{GlidePolar, PolarCoefficients, isa_density_ratio};
    use approx::assert_abs_diff_eq;
    use claims::{assert_ge, assert_le, assert_none, assert_some};
    use updraft_units::{Length, Mass, Speed};

    const SPEED_TOLERANCE: f64 = 0.05 / 3.6;

    fn polar() -> GlidePolar {
        let coefficients = assert_some!(PolarCoefficients::new(0.001, -0.04, 1.));
        assert_some!(GlidePolar::new(coefficients, Mass::from_kilograms(360.)))
    }

    fn speed(value: f64) -> Speed {
        Speed::from_meters_per_second(value)
    }

    #[test]
    fn calm_air_uses_the_analytic_maccready_speed_and_physical_sink() {
        let polar = polar();
        let distance = Length::from_kilometers(30.);
        for mc in [0_f64, 1., 3.] {
            let expected = speed(((1. + mc) / 0.001).sqrt());
            let mc = speed(mc);
            let glide = polar.solve_glide(distance, Length::ZERO, mc, Speed::ZERO, Speed::ZERO);
            let glide = assert_some!(glide);
            assert_abs_diff_eq!(glide.true_air_speed, expected, epsilon = SPEED_TOLERANCE);
            assert_eq!(glide.ground_speed, glide.true_air_speed);
            let sink = polar.sink_rate(glide.true_air_speed);
            let expected_loss = distance.as_meters() * sink / glide.ground_speed;
            assert_abs_diff_eq!(glide.height_loss.as_meters(), expected_loss, epsilon = 1e-9);
        }
    }

    #[test]
    fn along_track_wind_matches_the_analytic_solution() {
        let polar = polar();
        let distance = Length::from_kilometers(10.);
        for tailwind in [-20., 0., 20.] {
            for mc in [0., 2.] {
                let mc = speed(mc);
                let tailwind = speed(tailwind);
                let glide = polar.solve_glide(distance, Length::ZERO, mc, tailwind, Speed::ZERO);
                let glide = assert_some!(glide);
                let expected = polar.speed_to_fly(mc, Speed::ZERO, -tailwind);
                assert_abs_diff_eq!(glide.true_air_speed, expected, epsilon = SPEED_TOLERANCE);
            }
        }
    }

    #[test]
    fn density_scales_true_speed_and_sink_without_changing_calm_glide_ratio() {
        let polar = polar()
            .with_total_mass(Mass::from_kilograms(450.))
            .with_bugs(0.1);
        let altitude = Length::from_meters(3000.);
        let distance = Length::from_kilometers(10.);
        let glide = polar.solve_glide(distance, altitude, Speed::ZERO, Speed::ZERO, Speed::ZERO);
        let glide = assert_some!(glide);
        let expected = polar.best_glide_speed() / isa_density_ratio(altitude).sqrt();
        assert_abs_diff_eq!(glide.true_air_speed, expected, epsilon = SPEED_TOLERANCE);
        let expected_loss = distance / polar.best_glide_ratio();
        assert_abs_diff_eq!(glide.height_loss, expected_loss, epsilon = 0.001);
    }

    #[test]
    fn speed_ceiling_and_wind_feasibility_are_distinct() {
        let polar = polar();
        let distance = Length::from_kilometers(10.);
        let limit = Speed::from_kilometers_per_hour(400.);
        let mc = speed(1000.);
        let glide = polar.solve_glide(distance, Length::ZERO, mc, Speed::ZERO, Speed::ZERO);
        let glide = assert_some!(glide);
        assert_eq!(glide.true_air_speed, limit);
        for (tailwind, crosswind) in [
            (-limit, Speed::ZERO),
            (Speed::ZERO, limit),
            (speed(-80.), speed(80.)),
            (speed(50.), speed(120.)),
        ] {
            let glide = polar.solve_glide(distance, Length::ZERO, Speed::ZERO, tailwind, crosswind);
            assert_none!(glide);
        }
        let tailwind = speed(10.);
        let glide = polar.solve_glide(distance, Length::ZERO, Speed::ZERO, tailwind, limit);
        let glide = assert_some!(glide);
        assert_eq!(glide.true_air_speed, limit);
        assert_eq!(glide.ground_speed, speed(10.));
    }

    #[test]
    fn crosswind_solution_matches_a_speed_grid_in_both_directions() {
        let polar = polar();
        let distance = Length::from_kilometers(10.);
        let limit = Speed::from_kilometers_per_hour(400.).as_meters_per_second();
        for (tailwind, crosswind) in [
            (-30., 20.),
            (20., 40.),
            (-50., 95.),
            (10., 111.),
            (1000., 10.),
        ] {
            for mc in [0., 1., 10.] {
                let mac_cready = speed(mc);
                let wind = speed(tailwind);
                let solve = |crosswind| {
                    polar.solve_glide(distance, Length::ZERO, mac_cready, wind, speed(crosswind))
                };
                let glide = assert_some!(solve(crosswind));
                assert_eq!(assert_some!(solve(-crosswind)), glide);
                assert_ge!(glide.true_air_speed, polar.min_sink_speed());
                assert_le!(glide.true_air_speed.as_meters_per_second(), limit);
                let tas = glide.true_air_speed.as_meters_per_second();
                let ground = (tas * tas - crosswind * crosswind).sqrt() + tailwind;
                assert_abs_diff_eq!(glide.ground_speed, speed(ground), epsilon = 1e-9);
                let sink = polar.sink_rate(glide.true_air_speed).as_meters_per_second();
                let objective = (sink + mc) / ground;
                for step in 0..=1000 {
                    let candidate = 20. + (limit - 20.) * f64::from(step) / 1000.;
                    if candidate <= crosswind {
                        continue;
                    }
                    let ground = (candidate * candidate - crosswind * crosswind).sqrt() + tailwind;
                    if ground <= 0. {
                        continue;
                    }
                    let sink = polar.sink_rate(speed(candidate)).as_meters_per_second();
                    assert_le!(objective, (sink + mc) / ground * (1. + 1e-6));
                }
            }
        }
    }

    #[test]
    fn invalid_inputs_and_an_empty_speed_range_have_no_solution() {
        let polar = polar();
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for field in 0..5 {
                let mut values = [1000., 0., 0., 0., 0.];
                values[field] = invalid;
                let [distance, altitude, mc, tailwind, crosswind] = values;
                let distance = Length::from_meters(distance);
                let altitude = Length::from_meters(altitude);
                let [mc, tailwind, crosswind] = [mc, tailwind, crosswind].map(speed);
                let glide = polar.solve_glide(distance, altitude, mc, tailwind, crosswind);
                assert_none!(glide);
            }
        }
        for (distance, altitude, mc) in [(-1., 0., 0.), (1000., 0., -1.), (1000., 50_000., 0.)] {
            let distance = Length::from_meters(distance);
            let altitude = Length::from_meters(altitude);
            let glide = polar.solve_glide(distance, altitude, speed(mc), Speed::ZERO, Speed::ZERO);
            assert_none!(glide);
        }
        let heavy = polar.with_total_mass(Mass::from_kilograms(1_000_000.));
        let distance = Length::from_meters(1000.);
        let altitude = Length::ZERO;
        let glide = heavy.solve_glide(distance, altitude, Speed::ZERO, Speed::ZERO, Speed::ZERO);
        assert_none!(glide);
        let distance = Length::ZERO;
        let zero = polar.solve_glide(distance, altitude, Speed::ZERO, Speed::ZERO, Speed::ZERO);
        let zero = assert_some!(zero);
        assert_eq!(zero.height_loss, Length::ZERO);
    }
}
