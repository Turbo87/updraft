use crate::UtcInstant;
use updraft_geo::LatLon;
use updraft_units::{Length, Speed};

/// A step faster than this can be a position jump.
const MAX_SPEED: Speed = Speed::from_meters_per_second(150.);

/// A step shorter than this is never a position jump.
const MIN_DISTANCE: Length = Length::from_kilometers(2.);

/// The check ignores a longer fix-UTC step, because the recording rules
/// handle it.
const MAX_INTERVAL_MILLISECONDS: i64 = 3 * 60 * 60 * 1000;

/// The plausible fixes after a candidate that confirm it.
const CONFIRMATIONS: u8 = 3;

#[derive(Clone, Copy, Debug)]
struct TimedPosition {
    utc: UtcInstant,
    position: LatLon,
}

#[derive(Clone, Copy, Debug)]
struct Candidate {
    last: TimedPosition,
    confirmations: u8,
}

/// Checks the fixes of the selected GPS for a position jump.
///
/// A candidate is a fix that no glider can reach from the last accepted
/// fix. The next fixes confirm the candidate when each one follows
/// plausibly from the one before. Otherwise, a fix that follows plausibly
/// from the last accepted fix cancels the candidate.
#[derive(Clone, Copy, Debug, Default)]
pub struct PositionJumpCheck {
    accepted: Option<TimedPosition>,
    candidate: Option<Candidate>,
}

impl PositionJumpCheck {
    /// Returns whether the fix confirms a position jump.
    pub fn observe(&mut self, utc: UtcInstant, position: LatLon) -> bool {
        let fix = TimedPosition { utc, position };
        let Some(accepted) = self.accepted else {
            self.accept(fix);
            return false;
        };
        let last = self.candidate.map_or(accepted, |candidate| candidate.last);
        if utc == last.utc {
            return false;
        }
        let confirmations = match (self.candidate, Step::between(last, fix)) {
            (_, Step::Unchecked) | (None, Step::Plausible) => {
                self.accept(fix);
                return false;
            }
            (Some(candidate), Step::Plausible) => candidate.confirmations + 1,
            (Some(_), Step::Implausible) if Step::between(accepted, fix) == Step::Plausible => {
                self.accept(fix);
                return false;
            }
            (_, Step::Implausible) => 0,
        };
        if confirmations == CONFIRMATIONS {
            self.accept(fix);
            return true;
        }
        self.candidate = Some(Candidate {
            last: fix,
            confirmations,
        });
        false
    }

    fn accept(&mut self, fix: TimedPosition) {
        self.accepted = Some(fix);
        self.candidate = None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// The fix UTC does not advance, or advances by more than 3 h.
    Unchecked,
    Plausible,
    Implausible,
}

impl Step {
    fn between(from: TimedPosition, to: TimedPosition) -> Self {
        let interval = to.utc.unix_milliseconds() - from.utc.unix_milliseconds();
        if !(1..=MAX_INTERVAL_MILLISECONDS).contains(&interval) {
            return Self::Unchecked;
        }
        let distance = from.position.distance(to.position);
        let speed = Speed::from_meters_per_second(distance.as_meters() * 1_000. / interval as f64);
        match distance > MIN_DISTANCE && speed > MAX_SPEED {
            true => Self::Implausible,
            false => Self::Plausible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-01-01T12:00:00Z
    const UTC: i64 = 1_767_268_800_000;
    const HOURS: i64 = 60 * 60 * 1000;

    fn observe(check: &mut PositionJumpCheck, utc: i64, latitude_degrees: f64) -> bool {
        let utc = UtcInstant::from_unix_milliseconds(UTC + utc);
        check.observe(utc, LatLon::from_degrees(latitude_degrees, 6.0))
    }

    /// Observes one fix per second and returns the fixes that confirm a
    /// jump.
    fn jumps(latitudes: &[f64]) -> Vec<usize> {
        jumps_with_interval(1_000, latitudes)
    }

    fn jumps_with_interval(milliseconds: i64, latitudes: &[f64]) -> Vec<usize> {
        let mut check = PositionJumpCheck::default();
        (0..latitudes.len())
            .filter(|&index| observe(&mut check, index as i64 * milliseconds, latitudes[index]))
            .collect()
    }

    #[test]
    fn third_plausible_fix_after_the_jump_confirms_it() {
        assert_eq!(jumps(&[50.0, 50.0, 51.0, 51.0, 51.0, 51.0, 51.0]), [5]);
    }

    #[test]
    fn one_bad_fix_is_not_a_jump() {
        assert_eq!(
            jumps(&[50.0, 50.0, 51.0, 50.0, 50.0, 50.0, 50.0]),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn return_to_the_old_track_cancels_the_candidate() {
        assert_eq!(
            jumps(&[50.0, 51.0, 51.0, 51.0, 50.0, 50.0, 50.0]),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn implausible_step_after_the_jump_starts_a_new_candidate() {
        assert_eq!(jumps(&[50.0, 51.0, 51.0, 52.0, 52.0, 52.0, 52.0]), [6]);
    }

    #[test]
    fn jump_at_a_low_fix_rate_is_confirmed() {
        // 0.027° is about 3 km. The fix 30 s after the old track follows
        // from both tracks.
        let latitudes = [50.0, 50.027, 50.027, 50.027, 50.027];
        assert_eq!(jumps_with_interval(10_000, &latitudes), [4]);
    }

    #[test]
    fn short_fast_change_is_not_a_jump() {
        // 0.017° is about 1.9 km.
        assert_eq!(
            jumps(&[50.0, 50.017, 50.017, 50.017, 50.017]),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn long_slow_change_is_not_a_jump() {
        let mut check = PositionJumpCheck::default();
        observe(&mut check, 0, 50.0);
        // 0.1° is about 11 km, which is 111 m/s after 100 s.
        for (index, latitude) in [50.1, 50.1, 50.1, 50.1].into_iter().enumerate() {
            assert!(!observe(
                &mut check,
                100_000 + index as i64 * 1_000,
                latitude
            ));
        }
    }

    #[test]
    fn fix_utc_that_does_not_advance_is_not_checked() {
        let mut check = PositionJumpCheck::default();
        observe(&mut check, 0, 50.0);
        for _ in 0..4 {
            assert!(!observe(&mut check, 0, 51.0));
        }
        for second in 1..=4 {
            assert!(!observe(&mut check, second * 1_000, 50.0));
        }
    }

    #[test]
    fn backward_fix_utc_restarts_the_check() {
        const DAY: i64 = 24 * HOURS;
        let mut check = PositionJumpCheck::default();
        observe(&mut check, 0, 50.0);
        assert!(!observe(&mut check, -DAY, 50.0));
        for second in 1..=3 {
            assert!(!observe(&mut check, -DAY + second * 1_000, 51.0));
        }
        assert!(observe(&mut check, -DAY + 4_000, 51.0));
    }

    #[test]
    fn fix_utc_that_advances_more_than_3_hours_is_not_checked() {
        let mut check = PositionJumpCheck::default();
        observe(&mut check, 0, 50.0);
        let utc = 3 * HOURS + 1;
        for second in 0..4 {
            assert!(!observe(&mut check, utc + second * 1_000, 51.0));
        }
    }

    #[test]
    fn fix_utc_that_advances_3_hours_is_checked() {
        let mut check = PositionJumpCheck::default();
        observe(&mut check, 0, 50.0);
        // The antipode is about 20,000 km away, which needs about 1.9 km/s.
        let far = |check: &mut PositionJumpCheck, utc: i64| {
            let utc = UtcInstant::from_unix_milliseconds(UTC + utc);
            check.observe(utc, LatLon::from_degrees(-50.0, -174.0))
        };
        assert!(!far(&mut check, 3 * HOURS));
        assert!(!far(&mut check, 3 * HOURS + 1_000));
        assert!(!far(&mut check, 3 * HOURS + 2_000));
        assert!(far(&mut check, 3 * HOURS + 3_000));
    }
}
