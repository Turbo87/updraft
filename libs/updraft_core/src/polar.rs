use serde::{Deserialize, Serialize};
use updraft_polar::{GlidePolar, POLAR_STORE};

/// A built-in polar identified by its name, independent of catalog order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PolarId(#[cfg_attr(feature = "ts", ts(type = "string"))] &'static str);

impl PolarId {
    pub fn all() -> impl Iterator<Item = Self> {
        POLAR_STORE.iter().map(|entry| Self(entry.name))
    }

    pub fn name(self) -> &'static str {
        self.0
    }

    pub fn glide_polar(self) -> GlidePolar {
        POLAR_STORE
            .iter()
            .find(|entry| entry.name == self.0)
            .expect("polar IDs refer to built-in entries")
            .glide_polar()
    }
}

impl<'de> Deserialize<'de> for PolarId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl Default for PolarId {
    fn default() -> Self {
        Self("LS 8")
    }
}

impl TryFrom<String> for PolarId {
    type Error = UnknownPolar;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        Self::all()
            .find(|polar| polar.0 == name)
            .ok_or(UnknownPolar)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Unknown polar")]
pub struct UnknownPolar;

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use std::collections::BTreeSet;
    use updraft_units::Mass;

    #[test]
    fn default_is_the_15m_ls8_at_reference_mass() {
        let polar = PolarId::default();
        assert_eq!(polar.name(), "LS 8");
        assert_eq!(polar.glide_polar().total_mass(), Mass::from_kilograms(336.));
    }

    #[test]
    fn catalog_names_are_unique_and_resolve() {
        let mut names = BTreeSet::new();
        for polar in PolarId::all() {
            assert!(names.insert(polar.name()));
            let resolved = assert_ok!(PolarId::try_from(polar.name().to_owned()));
            assert_eq!(resolved, polar);
        }
    }

    #[test]
    fn unknown_name_is_rejected() {
        assert_err!(PolarId::try_from("Unknown glider".to_owned()));
    }
}
