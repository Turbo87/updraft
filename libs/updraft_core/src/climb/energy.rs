use super::velocity::Velocity;
use std::time::Duration;
use updraft_units::Length;

/// Accumulates height changes without adding offsets when compensation changes.
#[derive(Debug, Default)]
pub struct EnergyClimb {
    previous: Option<(Duration, Length)>,
    endpoint: Option<(Duration, Length, Option<Velocity>)>,
    height: Length,
}

impl EnergyClimb {
    /// A velocity endpoint must lie within the latest altitude-report interval.
    /// Midpoint endpoints leave the rest of that interval pending until the next report.
    pub fn observe(
        &mut self,
        time: Duration,
        altitude: Length,
        velocity: Option<(Duration, Velocity)>,
        wind: Option<Velocity>,
    ) -> Option<(Duration, Length)> {
        let previous = self.previous.unwrap_or((time, altitude));
        if self.previous.is_some() && time <= previous.0 {
            return None;
        }
        let velocity =
            velocity.filter(|(at, _)| wind.is_some() && *at >= previous.0 && *at <= time);
        self.previous = Some((time, altitude));
        let (at, altitude, velocity) = match velocity {
            Some((at, velocity)) if time > previous.0 => {
                let fraction = (at - previous.0).div_duration_f64(time - previous.0);
                (
                    at,
                    previous.1 + (altitude - previous.1) * fraction,
                    Some(velocity),
                )
            }
            _ => (time, altitude, velocity.map(|(_, velocity)| velocity)),
        };
        let endpoint = self.endpoint.unwrap_or((previous.0, previous.1, None));
        if at <= endpoint.0 && self.endpoint.is_some() {
            return None;
        }
        self.height += altitude - endpoint.1;
        if let Some(((previous, current), wind)) = endpoint.2.zip(velocity).zip(wind) {
            self.height += previous.energy_change(current, wind);
        }
        self.endpoint = Some((at, altitude, velocity));
        Some((at, self.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use claims::{assert_none, assert_some};
    use updraft_units::{Angle, Speed};

    fn observe(
        climb: &mut EnergyClimb,
        second: u64,
        altitude: f64,
        velocity_at: Option<u64>,
    ) -> (Duration, Length) {
        let velocity =
            velocity_at.map(|millis| (Duration::from_millis(millis), Velocity::default()));
        assert_some!(climb.observe(
            Duration::from_secs(second),
            Length::from_meters(altitude),
            velocity,
            Some(Velocity::default())
        ))
    }

    #[test]
    fn aligns_midpoints_and_flushes_pending_altitude_once() {
        let mut climb = EnergyClimb::default();
        observe(&mut climb, 0, 100., None);
        assert_eq!(
            observe(&mut climb, 2, 104., Some(1000)),
            (Duration::from_secs(1), Length::from_meters(2.))
        );
        assert_eq!(
            observe(&mut climb, 4, 108., Some(3000)),
            (Duration::from_secs(3), Length::from_meters(6.))
        );
        assert_eq!(
            observe(&mut climb, 6, 112., None),
            (Duration::from_secs(6), Length::from_meters(12.))
        );
        assert_eq!(
            observe(&mut climb, 8, 116., Some(7000)),
            (Duration::from_secs(7), Length::from_meters(14.))
        );
        assert_eq!(
            observe(&mut climb, 10, 120., Some(10000)),
            (Duration::from_secs(10), Length::from_meters(20.))
        );
        assert_none!(climb.observe(Duration::from_secs(9), Length::ZERO, None, None));
    }

    #[test]
    fn feeds_both_averagers_and_restarts_without_an_energy_offset() {
        use crate::climb::{ClimbEma, ClimbWindow};
        let mut climb = EnergyClimb::default();
        let mut window = ClimbWindow::default();
        let mut ema = ClimbEma::default();
        for second in 0..4 {
            let (time, height) = observe(
                &mut climb,
                second,
                100. + 2. * second as f64,
                Some(second * 1000),
            );
            window.observe(time, height);
            let average = ema.observe(time, height);
            if second > 0 {
                assert_eq!(assert_some!(average), Speed::from_meters_per_second(2.));
                assert_eq!(
                    assert_some!(window.average(Duration::from_secs(20))),
                    Speed::from_meters_per_second(2.)
                );
            }
        }
        climb = EnergyClimb::default();
        assert_eq!(observe(&mut climb, 90, 900., Some(90_000)).1, Length::ZERO);
        assert_eq!(
            observe(&mut climb, 92, 904., Some(92_000)).1,
            Length::from_meters(4.)
        );
    }

    #[test]
    fn recovery_does_not_bridge_energy_across_raw_intervals() {
        let mut climb = EnergyClimb::default();
        let mut heights = Vec::new();
        for (second, velocity_at, speed, wind) in [
            (0, 0, Some(40.), 0.),
            (1, 1000, Some(30.), 0.),
            (2, 2000, None, 0.),
            (3, 2500, Some(80.), 0.),
            (4, 4000, Some(80.), 20.),
            (5, 4500, Some(70.), 20.),
        ] {
            let velocity = speed.map(|speed| {
                (
                    Duration::from_millis(velocity_at),
                    Velocity::from_track(Angle::ZERO, Speed::from_meters_per_second(speed)),
                )
            });
            let wind = Velocity::from_track(Angle::ZERO, Speed::from_meters_per_second(wind));
            let (_, height) = assert_some!(climb.observe(
                Duration::from_secs(second),
                Length::ZERO,
                velocity,
                Some(wind)
            ));
            heights.push(height);
        }
        let first_loss = Length::from_meters(-700. / (2. * 9.80665));
        for height in &heights[1..5] {
            assert_abs_diff_eq!(*height, first_loss, epsilon = 1e-12);
        }
        assert_abs_diff_eq!(
            heights[5],
            Length::from_meters(-1800. / (2. * 9.80665)),
            epsilon = 1e-12
        );
    }

    #[test]
    fn cancels_height_gained_from_deceleration_and_falls_back_without_wind() {
        let mut climb = EnergyClimb::default();
        let fast = Velocity::from_track(Angle::ZERO, Speed::from_meters_per_second(40.));
        let slow = Velocity::from_track(Angle::ZERO, Speed::from_meters_per_second(30.));
        let wind = Some(Velocity::default());
        let gain = Length::from_meters(700. / (2. * 9.80665));
        climb.observe(
            Duration::ZERO,
            Length::ZERO,
            Some((Duration::ZERO, fast)),
            wind,
        );
        let (time, height) = assert_some!(climb.observe(
            Duration::from_secs(2),
            gain,
            Some((Duration::from_secs(2), slow)),
            wind
        ));
        assert_eq!(time, Duration::from_secs(2));
        assert_abs_diff_eq!(height, Length::ZERO, epsilon = 1e-12);
        let (_, raw) = assert_some!(climb.observe(
            Duration::from_secs(3),
            gain + Length::from_meters(2.),
            None,
            None
        ));
        assert_abs_diff_eq!(raw, Length::from_meters(2.), epsilon = 1e-12);
    }
}
