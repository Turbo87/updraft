use crate::{GlideSnapshot, Navigation, NavigationTarget, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedPinnedTarget {
    pub id: u32,
    pub target: NavigationTarget,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PinnedTarget {
    pub id: u32,
    pub navigation: Navigation,
    pub primary: bool,
}

#[derive(Debug, Default)]
pub struct PinnedTargets {
    targets: Vec<SavedPinnedTarget>,
    next_id: u32,
}

impl PinnedTargets {
    pub fn restore(&mut self, targets: Vec<SavedPinnedTarget>) -> Result<(), &'static str> {
        for (index, pin) in targets.iter().enumerate() {
            pin.target.validate()?;
            if targets[..index]
                .iter()
                .any(|other| other.id == pin.id || other.target.matches(&pin.target))
            {
                return Err("Duplicate pinned target");
            }
        }
        let next_id = targets
            .iter()
            .map(|pin| pin.id)
            .max()
            .map_or(Some(0), |id| id.checked_add(1))
            .ok_or("Pinned target IDs exhausted")?;
        self.targets = targets;
        self.next_id = self.next_id.max(next_id);
        Ok(())
    }

    pub fn pin(
        &mut self,
        target: NavigationTarget,
    ) -> Result<Vec<SavedPinnedTarget>, &'static str> {
        target.validate()?;
        if !self.targets.iter().any(|pin| pin.target.matches(&target)) {
            let id = self.next_id;
            self.next_id = id.checked_add(1).ok_or("Pinned target IDs exhausted")?;
            self.targets.push(SavedPinnedTarget { id, target });
        }
        Ok(self.targets.clone())
    }

    pub fn unpin(&mut self, id: u32) -> Vec<SavedPinnedTarget> {
        self.targets.retain(|pin| pin.id != id);
        self.targets.clone()
    }

    pub fn published(
        &self,
        glide: &GlideSnapshot,
        primary: Option<&NavigationTarget>,
        at: Timestamp,
    ) -> Vec<PinnedTarget> {
        self.targets
            .iter()
            .map(|pin| PinnedTarget {
                id: pin.id,
                primary: primary.is_some_and(|target| target.matches(&pin.target)),
                navigation: Navigation::new(pin.target.clone(), glide, None, None, at),
            })
            .collect()
    }
}
