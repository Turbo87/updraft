//! Stable identities for configured external devices.

/// A stable device ID preserved when an external device is edited or reordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeviceId(u64);

impl DeviceId {
    /// Creates a device identity from its persisted numeric value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}
