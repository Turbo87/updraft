/// Stores an unavailable, current, or last-known signal value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SignalState<T> {
    #[default]
    Unavailable,
    Current(T),
    LastKnown(T),
}
