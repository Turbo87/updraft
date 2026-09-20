use crate::NavigationTarget;
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
                self.index(id)?;
                self.start_tracking(id)?;
            }
            TaskCommand::Resume => {
                let id = self.saved.current.ok_or("The task is empty")?;
                self.start_tracking(id)?;
            }
            TaskCommand::Stop => self.saved.status = TaskStatus::Stopped,
        }
        Ok(())
    }

    fn start_tracking(&mut self, id: u32) -> Result<(), &'static str> {
        if self.saved.points.len() < 2 {
            return Err("A task needs two points");
        }
        self.saved.current = Some(id);
        self.saved.status = TaskStatus::Running;
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
