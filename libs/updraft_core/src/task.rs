use crate::{NavigationTarget, Timestamp};
use updraft_geo::LatLon;
mod cylinder;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    #[default]
    Stopped,
    Running,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TaskPoint {
    pub id: u32,
    pub target: NavigationTarget,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub points: Vec<TaskPoint>,
    pub current: Option<u32>,
    pub status: TaskStatus,
    pub next_id: u32,
    pub start: Option<TaskTime>,
    pub finish: Option<TaskTime>,
    pub restart_allowed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TaskTime {
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub unix_milliseconds: Option<i64>,
}

#[derive(Clone, Copy, Debug)]
struct PositionReport {
    position: LatLon,
    at: Timestamp,
}

#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TaskCommand {
    Add { target: NavigationTarget },
    Move { id: u32, index: usize },
    Remove { id: u32 },
    Select { id: u32 },
    Resume,
    Stop,
}

#[derive(Debug, Default)]
pub struct TaskState {
    saved: Task,
    previous: Option<PositionReport>,
}

impl TaskState {
    pub fn snapshot(&self) -> Task {
        self.saved.clone()
    }

    pub fn target(&self) -> Option<&NavigationTarget> {
        self.saved
            .points
            .iter()
            .find(|point| Some(point.id) == self.saved.current)
            .map(|point| &point.target)
    }

    pub fn restore(&mut self, saved: Task) -> Result<(), &'static str> {
        for (index, point) in saved.points.iter().enumerate() {
            Self::validate_waypoint(&point.target)?;
            if point.id >= saved.next_id
                || saved.points[..index]
                    .iter()
                    .any(|other| other.id == point.id)
            {
                return Err("Invalid task point ID");
            }
        }
        if saved
            .current
            .is_some_and(|id| !saved.points.iter().any(|point| point.id == id))
            || (saved.status != TaskStatus::Stopped
                && (saved.points.len() < 2 || saved.current.is_none()))
        {
            return Err("Invalid task progress");
        }
        self.saved = saved;
        self.reset_crossing();
        Ok(())
    }

    fn validate_waypoint(target: &NavigationTarget) -> Result<(), &'static str> {
        if !matches!(target, NavigationTarget::Waypoint { .. }) {
            return Err("Task points must be waypoints");
        }
        target.validate()
    }

    pub fn change(&mut self, command: TaskCommand) -> Result<(), &'static str> {
        match command {
            TaskCommand::Add { target } => {
                Self::validate_waypoint(&target)?;
                let id = self.saved.next_id;
                self.saved.next_id = id.checked_add(1).ok_or("Task point IDs exhausted")?;
                self.saved.points.push(TaskPoint { id, target });
                if self.saved.current.is_none() {
                    self.saved.restart_allowed = true;
                }
                self.saved.current.get_or_insert(id);
            }
            TaskCommand::Move { id, index } => {
                if index >= self.saved.points.len() {
                    return Err("Invalid task point position");
                }
                let old = self.index(id)?;
                let point = self.saved.points.remove(old);
                self.saved.points.insert(index, point);
            }
            TaskCommand::Remove { id } => {
                let index = self.index(id)?;
                if self.saved.status == TaskStatus::Running && self.saved.points.len() <= 2 {
                    return Err("A running task needs two points");
                }
                self.saved.points.remove(index);
                if self.saved.current == Some(id) {
                    self.saved.current = self
                        .saved
                        .points
                        .get(index)
                        .or_else(|| self.saved.points.last())
                        .map(|point| point.id);
                }
                if self.saved.points.len() < 2 {
                    self.saved.status = TaskStatus::Stopped;
                }
            }
            TaskCommand::Select { id } => {
                let index = self.index(id)?;
                self.start_tracking(id)?;
                if index == 0 {
                    self.saved.restart_allowed = true;
                } else if index > 1 {
                    self.saved.restart_allowed = false;
                }
            }
            TaskCommand::Resume => {
                if self.saved.status == TaskStatus::Running {
                    return Ok(());
                }
                let id = self.saved.current.ok_or("The task is empty")?;
                self.start_tracking(id)?;
            }
            TaskCommand::Stop => self.saved.status = TaskStatus::Stopped,
        }
        self.reset_crossing();
        Ok(())
    }

    fn start_tracking(&mut self, id: u32) -> Result<(), &'static str> {
        if self.saved.points.len() < 2 {
            return Err("A task needs two points");
        }
        self.saved.current = Some(id);
        self.saved.status = TaskStatus::Running;
        self.saved.finish = None;
        Ok(())
    }

    fn index(&self, id: u32) -> Result<usize, &'static str> {
        self.saved
            .points
            .iter()
            .position(|point| point.id == id)
            .ok_or("Unknown task point")
    }
}

impl TaskState {
    pub fn reset_crossing(&mut self) {
        self.previous = None;
    }

    pub fn observe(&mut self, position: LatLon, at: Timestamp, utc: Option<i64>) {
        if self.saved.status != TaskStatus::Running {
            return;
        }
        if self.previous.is_some_and(|previous| at <= previous.at) {
            return;
        }
        let previous = self.previous.replace(PositionReport { position, at });
        let Some(previous) = previous else {
            return;
        };
        if at.saturating_since(previous.at).as_millis() > 10_000 {
            return;
        }
        let gap = at.saturating_since(previous.at).as_millis() as f64;
        let mut cursor = -1.;
        for _ in 0..=self.saved.points.len() {
            let Some(index) = self
                .saved
                .points
                .iter()
                .position(|point| Some(point.id) == self.saved.current)
            else {
                return;
            };
            let crossing = |index: usize, exit: bool| {
                let center = self.saved.points[index].target.position()?;
                let center =
                    LatLon::from_degrees(center.latitude_degrees, center.longitude_degrees);
                let (entry, leave) = cylinder::crossings(previous.position, position, center);
                (if exit { leave } else { entry }).filter(|fraction| *fraction > cursor + 1e-9)
            };
            let restart = self
                .saved
                .restart_allowed
                .then(|| crossing(0, true))
                .flatten();
            let entry = (index > 0).then(|| crossing(index, false)).flatten();
            let (fraction, reached) = match (restart, entry) {
                (Some(start), Some(point)) if start < point => (start, 0),
                (_, Some(point)) => (point, index),
                (Some(start), None) => (start, 0),
                _ => return,
            };
            cursor = fraction;
            let time = TaskTime {
                unix_milliseconds: utc
                    .map(|utc| utc.saturating_sub(((1. - fraction) * gap).round() as i64)),
            };
            if reached == 0 {
                self.saved.start = Some(time);
                self.saved.current = Some(self.saved.points[1].id);
            } else {
                self.saved.restart_allowed = false;
                if reached + 1 == self.saved.points.len() {
                    self.saved.finish = Some(time);
                    self.saved.status = TaskStatus::Completed;
                    self.reset_crossing();
                    return;
                }
                self.saved.current = Some(self.saved.points[reached + 1].id);
            }
        }
    }
}
