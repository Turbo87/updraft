use updraft_units::{Angle, Length, Speed};

#[derive(Clone, Copy, Debug, Default)]
pub struct Velocity {
    pub east: Speed,
    pub north: Speed,
}

impl Velocity {
    pub fn from_track(track: Angle, speed: Speed) -> Self {
        Self {
            east: speed * track.sin(),
            north: speed * track.cos(),
        }
    }

    /// Height equivalent of the airspeed energy change, using one wind estimate.
    pub fn energy_change(self, next: Self, wind: Self) -> Length {
        fn energy(ground: Velocity, wind: Velocity) -> Length {
            let east = (ground.east - wind.east).as_meters_per_second();
            let north = (ground.north - wind.north).as_meters_per_second();
            Length::from_meters((east * east + north * north) / (2. * 9.80665))
        }
        energy(next, wind) - energy(self, wind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn compensates_speed_changes_in_the_air_mass() {
        let previous = Velocity::from_track(Angle::ZERO, Speed::from_meters_per_second(40.));
        let current = Velocity::from_track(Angle::ZERO, Speed::from_meters_per_second(30.));
        let wind = Velocity::from_track(Angle::ZERO, Speed::from_meters_per_second(10.));
        let change = previous.energy_change(current, wind);
        assert_abs_diff_eq!(
            change,
            Length::from_meters(-500. / (2. * 9.80665)),
            epsilon = 1e-12
        );
        assert_eq!(previous.energy_change(previous, wind), Length::ZERO);
    }

    #[test]
    fn turns_at_constant_airspeed_do_not_change_energy() {
        let wind = Velocity {
            east: Speed::from_meters_per_second(5.),
            north: Speed::ZERO,
        };
        let previous = Velocity {
            east: Speed::from_meters_per_second(35.),
            north: Speed::ZERO,
        };
        let current = Velocity {
            east: wind.east,
            north: Speed::from_meters_per_second(30.),
        };
        assert_eq!(previous.energy_change(current, wind), Length::ZERO);
        let east =
            Velocity::from_track(Angle::from_degrees(90.), Speed::from_meters_per_second(30.));
        assert_abs_diff_eq!(
            east.east,
            Speed::from_meters_per_second(30.),
            epsilon = 1e-12
        );
        assert_abs_diff_eq!(east.north, Speed::ZERO, epsilon = 1e-12);
    }
}
