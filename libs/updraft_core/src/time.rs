use std::time::Duration;

/// Monotonic time since the shell started, supplied with every input.
///
/// The core never reads a clock. Time is always passed in, which is what
/// makes a scripted sequence of inputs reproduce exactly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(Duration);

impl Timestamp {
    pub const fn from_millis(millis: u64) -> Self {
        Self(Duration::from_millis(millis))
    }

    pub const fn as_millis(self) -> u64 {
        self.0.as_millis() as u64
    }

    /// Time elapsed since `earlier`, clamped at zero so a late or
    /// out-of-order input can never produce a negative duration.
    pub fn saturating_since(self, earlier: Timestamp) -> Duration {
        self.0.saturating_sub(earlier.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_elapsed_time_without_going_negative() {
        let earlier = Timestamp::from_millis(1_000);
        let later = Timestamp::from_millis(1_250);

        assert_eq!(later.saturating_since(earlier), Duration::from_millis(250));
        assert_eq!(earlier.saturating_since(later), Duration::ZERO);
    }
}
