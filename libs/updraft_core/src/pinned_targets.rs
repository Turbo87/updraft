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
    targets: Vec<PinState>,
    next_id: u32,
}

#[derive(Debug)]
struct PinState {
    saved: SavedPinnedTarget,
    elevation: Option<f64>,
    report: Option<(crate::PublishedTrafficTarget, Timestamp)>,
}

impl PinState {
    fn new(saved: SavedPinnedTarget) -> Self {
        Self {
            saved,
            elevation: None,
            report: None,
        }
    }
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
        self.targets = targets.into_iter().map(PinState::new).collect();
        self.next_id = self.next_id.max(next_id);
        Ok(())
    }

    pub fn pin(
        &mut self,
        target: NavigationTarget,
    ) -> Result<Vec<SavedPinnedTarget>, &'static str> {
        target.validate()?;
        if !self
            .targets
            .iter()
            .any(|pin| pin.saved.target.matches(&target))
        {
            let id = self.next_id;
            self.next_id = id.checked_add(1).ok_or("Pinned target IDs exhausted")?;
            self.targets
                .push(PinState::new(SavedPinnedTarget { id, target }));
        }
        Ok(self.saved())
    }

    pub fn unpin(&mut self, id: u32) -> Vec<SavedPinnedTarget> {
        self.targets.retain(|pin| pin.saved.id != id);
        self.saved()
    }

    fn saved(&self) -> Vec<SavedPinnedTarget> {
        self.targets.iter().map(|pin| pin.saved.clone()).collect()
    }

    pub fn report(
        &self,
        target: &NavigationTarget,
    ) -> Option<(crate::PublishedTrafficTarget, Timestamp)> {
        self.targets
            .iter()
            .find(|pin| pin.saved.target.matches(target))
            .and_then(|pin| pin.report.clone())
    }

    pub fn remember_report(
        &mut self,
        target: &NavigationTarget,
        report: &(crate::PublishedTrafficTarget, Timestamp),
    ) {
        for pin in &mut self.targets {
            if pin.report.is_none() && pin.saved.target.matches(target) {
                pin.report = Some(report.clone());
            }
        }
    }

    pub fn update_reports(
        &mut self,
        traffic: &crate::TrafficState,
        flarmnet: &updraft_flarmnet::FlarmnetDatabase,
    ) {
        for pin in &mut self.targets {
            if let NavigationTarget::Traffic { id } = pin.saved.target
                && let Some(report) = traffic.navigation_observation(id, flarmnet)
            {
                pin.report = Some(report);
            }
        }
    }

    pub fn set_elevation(&mut self, update: crate::PinnedTargetElevation) {
        if let Some(pin) = self
            .targets
            .iter_mut()
            .find(|pin| pin.saved.id == update.id)
            && matches!(pin.saved.target, NavigationTarget::MapPosition { .. })
            && pin.saved.target.position() == Some(update.position)
        {
            pin.elevation = update.meters.filter(|meters| meters.is_finite());
        }
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
                id: pin.saved.id,
                primary: primary.is_some_and(|target| target.matches(&pin.saved.target)),
                navigation: Navigation::new(
                    pin.saved.target.clone(),
                    glide,
                    pin.elevation,
                    pin.report.as_ref(),
                    at,
                ),
            })
            .collect()
    }
}
