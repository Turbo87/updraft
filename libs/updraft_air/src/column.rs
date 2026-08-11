use crate::smoothing_weight;

/// How long a block of flight has to last before it contributes, in
/// seconds.
const BLOCK: f64 = 30.;

/// How much pressure altitude a block has to cover, in metres. A ratio of
/// two small changes is mostly a ratio of two noises.
const MIN_CHANGE: f64 = 20.;

/// Time constant of the ratio, in seconds. One climb measures one thermal
/// and the air it grew in, so the ratio has to average over several.
const TIME_CONSTANT: f64 = 600.;

/// Ratios a real troposphere can reach, about 40 K either side of the
/// standard atmosphere.
const MIN_RATIO: f64 = 0.86;
const MAX_RATIO: f64 = 1.14;

/// Measures how much warmer the air column is than the standard one.
///
/// A pressure altitude follows the ISA. The real atmosphere gives
/// `dz/dHp = T_real/T_ISA`, so the geometric height a glider gains,
/// divided by the pressure altitude it gains over the same interval, is
/// that temperature ratio.
///
/// It matters because the air density at a pressure altitude is
/// `σ_ISA(Hp) / ratio`, and the density scales the glide polar. A column
/// 16 K above the ISA leaves the air about 5% thinner than the ISA says,
/// which the sink rate would otherwise miss.
///
/// The ratio comes from pairs of height *changes*, not from the offset
/// between the two altitudes. Both altitudes drift slowly against each
/// other, through the weather and through the receiver, and a difference
/// of changes over half a minute leaves that drift behind.
#[derive(Clone, Copy, Debug, Default)]
pub struct ColumnRatio {
    /// Time, geometric altitude and pressure altitude the current block
    /// started at.
    checkpoint: Option<(f64, f64, f64)>,
    /// Running `Σ dz·dHp` and `Σ dHp²`, which divide into the slope of a
    /// least-squares line through the origin.
    cross: f64,
    square: f64,
}

impl ColumnRatio {
    /// Takes a geometric and a pressure altitude measured at the same
    /// moment.
    pub fn update(&mut self, time: f64, geometric: f64, pressure: f64) {
        let Some((start, start_geometric, start_pressure)) = self.checkpoint else {
            self.checkpoint = Some((time, geometric, pressure));
            return;
        };

        let elapsed = time - start;
        if elapsed.is_nan() || elapsed < BLOCK {
            return;
        }
        self.checkpoint = Some((time, geometric, pressure));

        let pressure_change = pressure - start_pressure;
        if pressure_change.abs() < MIN_CHANGE {
            return;
        }
        let geometric_change = geometric - start_geometric;

        let weight = smoothing_weight(elapsed, TIME_CONSTANT);
        self.cross += weight * (geometric_change * pressure_change - self.cross);
        self.square += weight * (pressure_change * pressure_change - self.square);
    }

    /// The ratio of the column temperature to the standard one, or 1
    /// while no climb has been measured.
    pub fn ratio(&self) -> f64 {
        if self.square <= 0. {
            return 1.;
        }
        let ratio = self.cross / self.square;
        match ratio.is_nan() {
            true => 1.,
            false => ratio.clamp(MIN_RATIO, MAX_RATIO),
        }
    }

    /// Whether a climb has been measured, so that the ratio is more than
    /// the standard atmosphere's assumption.
    pub fn is_measured(&self) -> bool {
        self.square > 0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    /// Climbs and glides through a column `ratio` times the standard
    /// temperature, one sample per second.
    fn fly(ratio: f64, seconds: u64) -> ColumnRatio {
        let mut column = ColumnRatio::default();
        let mut pressure = 1000.;
        let mut geometric = 1500.;
        for second in 0..seconds {
            // Three minutes of climb, then three of glide.
            let rate = match (second / 180) % 2 {
                0 => 2.,
                _ => -1.,
            };
            pressure += rate;
            geometric += rate * ratio;
            column.update(second as f64, geometric, pressure);
        }
        column
    }

    #[test]
    fn a_standard_column_reads_one() {
        assert_abs_diff_eq!(fly(1., 3600).ratio(), 1., epsilon = 1e-6);
    }

    #[test]
    fn a_warm_column_reads_above_one() {
        assert_abs_diff_eq!(fly(1.057, 3600).ratio(), 1.057, epsilon = 1e-3);
    }

    #[test]
    fn a_cold_column_reads_below_one() {
        assert_abs_diff_eq!(fly(0.95, 3600).ratio(), 0.95, epsilon = 1e-3);
    }

    #[test]
    fn nothing_measured_reads_the_standard_atmosphere() {
        let column = ColumnRatio::default();

        assert!(!column.is_measured());
        assert_abs_diff_eq!(column.ratio(), 1., epsilon = 1e-9);
    }

    #[test]
    fn level_flight_measures_nothing() {
        let mut column = ColumnRatio::default();
        for second in 0..3600 {
            column.update(second as f64, 1500., 1000.);
        }

        assert!(!column.is_measured());
    }

    #[test]
    fn an_impossible_ratio_is_held_at_the_limit() {
        assert_abs_diff_eq!(fly(2., 3600).ratio(), MAX_RATIO, epsilon = 1e-9);
        assert_abs_diff_eq!(fly(0.1, 3600).ratio(), MIN_RATIO, epsilon = 1e-9);
    }
}
