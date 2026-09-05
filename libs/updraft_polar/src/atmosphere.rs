use updraft_units::Length;

/// Air density relative to sea level, using the ISA tropospheric approximation.
/// The temperature ratio is clamped to zero above the model's temperature limit.
pub fn isa_density_ratio(altitude: Length) -> f64 {
    const LAPSE_RATE: f64 = 2.255_77e-5;
    const EXPONENT: f64 = 4.255_88;

    let temperature_ratio = (1. - LAPSE_RATE * altitude.as_meters()).max(0.);
    temperature_ratio.powf(EXPONENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn the_isa_model_matches_the_published_density_ratios() {
        let at_1000m = isa_density_ratio(Length::from_meters(1000.));
        let at_3000m = isa_density_ratio(Length::from_meters(3000.));
        let at_5000m = isa_density_ratio(Length::from_meters(5000.));

        assert_abs_diff_eq!(isa_density_ratio(Length::ZERO), 1., epsilon = 1e-9);
        assert_abs_diff_eq!(at_1000m, 0.9075, epsilon = 0.0005);
        assert_abs_diff_eq!(at_3000m, 0.7423, epsilon = 0.0005);
        assert_abs_diff_eq!(at_5000m, 0.6012, epsilon = 0.0005);
    }
}
