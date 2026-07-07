use updraft_units::Speed;

/// Coefficients of the quadratic glide polar approximation.
///
/// The polar predicts a glider's still-air sink rate `w` at airspeed `v`
/// as `w(v) = a·v² + b·v + c`, with both speeds in meters per second.
///
/// Sink rates in this crate are positive numbers: a glider descending
/// at 0.7 m/s has a sink rate of `0.7`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolarCoefficients {
    a: f64,
    b: f64,
    c: f64,
}

impl PolarCoefficients {
    /// Creates coefficients from raw values, or `None` unless they
    /// describe a physically meaningful polar: `a > 0` and `b < 0`, so
    /// the sink parabola has its minimum at a positive airspeed, the
    /// minimum sink rate is positive (a glider always descends in still
    /// air), and all values are finite. All derived values (minimum
    /// sink, best glide, speed to fly) rely on these invariants.
    pub fn new(a: f64, b: f64, c: f64) -> Option<Self> {
        let coefficients = Self { a, b, c };
        let valid = [a, b, c].iter().all(|value| value.is_finite())
            && a > 0.
            && b < 0.
            && coefficients.min_sink_rate() > Speed::ZERO;
        valid.then_some(coefficients)
    }

    /// Fits the quadratic through three measured `(airspeed, sink rate)`
    /// points, e.g. from a flight manual or a WinPilot-style polar file.
    ///
    /// Sink rates must be positive numbers. WinPilot-style polar files
    /// quote the points as negative vertical speeds, so negate those
    /// values when importing. Returns `None` if the speeds are not
    /// distinct or the resulting parabola is not a valid polar (e.g.
    /// the points curve the wrong way).
    pub fn from_points(points: [(Speed, Speed); 3]) -> Option<Self> {
        let [v1, v2, v3] = points.map(|(v, _)| v.as_meters_per_second());
        let [w1, w2, w3] = points.map(|(_, w)| w.as_meters_per_second());

        // Newton's divided differences for the interpolating parabola.
        let d1 = (w2 - w1) / (v2 - v1);
        let d2 = (w3 - w2) / (v3 - v2);
        let a = (d2 - d1) / (v3 - v1);
        let b = d1 - a * (v1 + v2);
        let c = w1 - (a * v1 + b) * v1;

        Self::new(a, b, c)
    }

    /// The `v²` coefficient (s/m).
    pub fn a(self) -> f64 {
        self.a
    }

    /// The `v` coefficient (dimensionless).
    pub fn b(self) -> f64 {
        self.b
    }

    /// The constant coefficient (m/s).
    pub fn c(self) -> f64 {
        self.c
    }

    /// The still-air sink rate (positive) at the given airspeed.
    pub fn sink_rate(self, air_speed: Speed) -> Speed {
        let v = air_speed.as_meters_per_second();
        Speed::from_meters_per_second((self.a * v + self.b) * v + self.c)
    }

    /// The airspeed of minimum sink.
    pub fn min_sink_speed(self) -> Speed {
        Speed::from_meters_per_second(-self.b / (2. * self.a))
    }

    /// The sink rate (positive) at minimum sink speed.
    pub fn min_sink_rate(self) -> Speed {
        Speed::from_meters_per_second(self.c - self.b * self.b / (4. * self.a))
    }

    /// The airspeed of best glide in still air, where `w(v)/v` is
    /// minimal: `v = √(c/a)`.
    pub fn best_glide_speed(self) -> Speed {
        Speed::from_meters_per_second((self.c / self.a).sqrt())
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

    #[test]
    fn fit_reproduces_points() {
        let points = [
            (kmh(100.), mps(0.67)),
            (kmh(155.), mps(1.45)),
            (kmh(185.), mps(2.5)),
        ];
        let polar = PolarCoefficients::from_points(points).unwrap();
        for (speed, sink) in points {
            assert_abs_diff_eq!(polar.sink_rate(speed), sink);
        }
    }

    #[test]
    fn fit_is_order_independent() {
        // The interpolating parabola is the same for any point order (up
        // to floating-point rounding).
        let a = (kmh(100.), mps(0.67));
        let b = (kmh(155.), mps(1.45));
        let c = (kmh(185.), mps(2.5));
        let fitted = PolarCoefficients::from_points([a, b, c]).unwrap();
        let permuted = PolarCoefficients::from_points([c, a, b]).unwrap();
        for speed in [80., 120., 160., 200.] {
            assert_abs_diff_eq!(
                permuted.sink_rate(kmh(speed)),
                fitted.sink_rate(kmh(speed)),
                epsilon = 1e-15
            );
        }
    }

    #[test]
    fn rejects_invalid_polars() {
        assert!(PolarCoefficients::new(-1e-3, -0.2, 3.).is_none());
        assert!(PolarCoefficients::new(1e-3, 0.2, 3.).is_none());
        assert!(PolarCoefficients::new(f64::INFINITY, -0.2, 3.).is_none());
        assert!(PolarCoefficients::new(1e-3, f64::NEG_INFINITY, 3.).is_none());
        assert!(PolarCoefficients::new(1e-3, -0.2, f64::NAN).is_none());

        // Duplicate speeds cannot be fitted.
        let duplicate = [
            (kmh(100.), mps(0.67)),
            (kmh(100.), mps(1.45)),
            (kmh(185.), mps(2.5)),
        ];
        assert!(PolarCoefficients::from_points(duplicate).is_none());

        // Sink flattening out at high speed curves the wrong way (a < 0).
        let concave = [
            (kmh(100.), mps(1.)),
            (kmh(140.), mps(1.5)),
            (kmh(180.), mps(1.8)),
        ];
        assert!(PolarCoefficients::from_points(concave).is_none());

        // Published polar files quote sink as negative vertical speed.
        // Passing those values unnegated must not produce a "climbing"
        // polar.
        let unnegated = [
            (kmh(100.), mps(-0.67)),
            (kmh(155.), mps(-1.45)),
            (kmh(185.), mps(-2.5)),
        ];
        assert!(PolarCoefficients::from_points(unnegated).is_none());
    }

    #[test]
    fn derived_values() {
        // LS8 15m: min sink 0.67 m/s at ~99 km/h, best glide ~44 at ~112 km/h.
        let polar = PolarCoefficients::from_points([
            (kmh(100.), mps(0.67)),
            (kmh(155.), mps(1.45)),
            (kmh(185.), mps(2.5)),
        ])
        .unwrap();

        let min_sink_speed = polar.min_sink_speed();
        assert_abs_diff_eq!(min_sink_speed.as_kilometers_per_hour(), 98.6, epsilon = 0.1);
        let min_sink_rate = polar.min_sink_rate();
        assert_abs_diff_eq!(min_sink_rate.as_meters_per_second(), 0.67, epsilon = 0.01);

        let best_glide_speed = polar.best_glide_speed();
        assert_abs_diff_eq!(
            best_glide_speed.as_kilometers_per_hour(),
            111.6,
            epsilon = 0.1
        );

        // The minimum of the parabola is indeed at min sink speed.
        let delta = mps(0.1);
        assert!(polar.sink_rate(min_sink_speed) < polar.sink_rate(min_sink_speed - delta));
        assert!(polar.sink_rate(min_sink_speed) < polar.sink_rate(min_sink_speed + delta));

        // Best glide speed maximizes the glide ratio.
        let ratio = |v: Speed| v / polar.sink_rate(v);
        assert!(ratio(best_glide_speed) > ratio(best_glide_speed - delta));
        assert!(ratio(best_glide_speed) > ratio(best_glide_speed + delta));
        assert_abs_diff_eq!(ratio(best_glide_speed), 43.6, epsilon = 0.1);
    }
}
