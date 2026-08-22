/// Stores an unavailable, current, or last-known signal value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SignalState<T> {
    #[default]
    Unavailable,
    Current(T),
    LastKnown(T),
}

impl<T> SignalState<T> {
    /// Stores `value` as the current signal value.
    pub fn update(&mut self, value: T) {
        *self = Self::Current(value);
    }

    /// Marks a current value as last known without changing its value.
    pub fn mark_stale(&mut self) {
        let state = std::mem::take(self);
        *self = match state {
            Self::Current(value) => Self::LastKnown(value),
            state => state,
        };
    }

    /// Returns the available value and whether it is last known.
    pub fn value_with_stale(self) -> Option<(T, bool)> {
        match self {
            Self::Unavailable => None,
            Self::Current(value) => Some((value, false)),
            Self::LastKnown(value) => Some((value, true)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_some_eq;

    #[derive(Debug, PartialEq)]
    struct NonCopy(u8);

    #[test]
    fn update_makes_a_value_current() {
        let mut state = SignalState::Unavailable;

        state.update(42);

        assert_eq!(state, SignalState::Current(42));
    }

    #[test]
    fn mark_stale_preserves_the_last_current_value() {
        let mut state = SignalState::Current(42);

        state.mark_stale();
        state.mark_stale();

        assert_eq!(state, SignalState::LastKnown(42));
    }

    #[test]
    fn value_with_stale_projects_availability_and_freshness() {
        assert_eq!(SignalState::<u8>::Unavailable.value_with_stale(), None);
        assert_some_eq!(SignalState::Current(42).value_with_stale(), (42, false));
        assert_some_eq!(SignalState::LastKnown(42).value_with_stale(), (42, true));
    }

    #[test]
    fn transitions_support_non_copy_values() {
        let mut state = SignalState::Unavailable;

        state.update(NonCopy(42));
        state.mark_stale();

        assert_some_eq!(state.value_with_stale(), (NonCopy(42), true));
    }
}
