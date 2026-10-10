use crate::{NavigationTarget, UtcInstant};
use updraft_geo::LatLon;
mod cylinder;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TaskPoint {
    pub id: u32,
    pub target: NavigationTarget,
}

/// The route of the task. The task file stores only the route.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub points: Vec<TaskPoint>,
    pub next_id: u32,
}

/// The reached task points and the start and finish times.
///
/// Progress is derived from the route and the recorded fixes. The
/// navigation target of the task does not change it.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    /// The number of reached points. Points are reached in route order.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub reached: usize,
    pub start: Option<TaskTime>,
    pub finish: Option<TaskTime>,
}

/// The route of the task with its derived state.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PublishedTask {
    #[serde(flatten)]
    pub route: Task,
    /// The ID of the point that task navigation guides to.
    pub target: Option<u32>,
    pub progress: TaskProgress,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TaskTime {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub unix_milliseconds: i64,
}

#[derive(Clone, Copy, Debug)]
struct PositionReport {
    position: LatLon,
    utc: UtcInstant,
}

#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TaskCommand {
    Add { target: NavigationTarget },
    Move { id: u32, index: usize },
    Remove { id: u32 },
    Select { id: u32 },
    Stop,
}

#[derive(Debug, Default)]
pub struct TaskState {
    route: Task,
    /// The recorded fixes of the current recording since startup.
    fixes: Vec<PositionReport>,
    progress: TaskProgress,
    target: Option<u32>,
}

impl TaskState {
    pub fn snapshot(&self) -> PublishedTask {
        PublishedTask {
            route: self.route.clone(),
            target: self.target,
            progress: self.progress.clone(),
        }
    }

    pub fn route(&self) -> &Task {
        &self.route
    }

    pub fn target(&self) -> Option<&NavigationTarget> {
        self.route
            .points
            .iter()
            .find(|point| Some(point.id) == self.target)
            .map(|point| &point.target)
    }

    pub fn restore(&mut self, route: Task) -> Result<(), &'static str> {
        for (index, point) in route.points.iter().enumerate() {
            Self::validate_waypoint(&point.target)?;
            if point.id >= route.next_id
                || route.points[..index]
                    .iter()
                    .any(|other| other.id == point.id)
            {
                return Err("Invalid task point ID");
            }
        }
        self.route = route;
        self.derive_progress();
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
                let id = self.route.next_id;
                self.route.next_id = id.checked_add(1).ok_or("Task point IDs exhausted")?;
                self.route.points.push(TaskPoint { id, target });
            }
            TaskCommand::Move { id, index } => {
                if index >= self.route.points.len() {
                    return Err("Invalid task point position");
                }
                let old = self.index(id)?;
                let point = self.route.points.remove(old);
                self.route.points.insert(index, point);
            }
            TaskCommand::Remove { id } => {
                let index = self.index(id)?;
                self.route.points.remove(index);
            }
            TaskCommand::Select { id } => {
                self.index(id)?;
                if self.route.points.len() < 2 {
                    return Err("A task needs two points");
                }
                self.target = Some(id);
                return Ok(());
            }
            TaskCommand::Stop => self.route = Task::default(),
        }
        self.derive_progress();
        Ok(())
    }

    fn index(&self, id: u32) -> Result<usize, &'static str> {
        self.route
            .points
            .iter()
            .position(|point| point.id == id)
            .ok_or("Unknown task point")
    }

    /// Derives the progress from all recorded fixes and targets the first
    /// unreached point.
    fn derive_progress(&mut self) {
        self.progress = TaskProgress::default();
        for segment in self.fixes.windows(2) {
            self.progress
                .advance(&self.route.points, segment[0], segment[1]);
        }
        self.target = match self.route.points.len() {
            0 | 1 => None,
            _ => self
                .route
                .points
                .get(self.progress.reached)
                .map(|point| point.id),
        };
    }

    /// Starts a new recording without fixes and progress.
    pub fn start_recording(&mut self) {
        self.fixes.clear();
        self.progress = TaskProgress::default();
    }

    /// Observes a fix that the flight recorder records. Returns whether
    /// the fix reached the finish while the finish was the target.
    pub fn observe(&mut self, position: LatLon, utc: UtcInstant) -> bool {
        let fix = PositionReport { position, utc };
        let previous = self.fixes.last().copied();
        self.fixes.push(fix);
        let Some(previous) = previous else {
            return false;
        };
        self.progress.advance(&self.route.points, previous, fix);
        self.advance_target(previous, fix)
    }

    fn advance_target(&mut self, from: PositionReport, to: PositionReport) -> bool {
        let points = &self.route.points;
        let mut cursor = -1.;
        loop {
            let Some(index) = points
                .iter()
                .position(|point| Some(point.id) == self.target)
            else {
                return false;
            };
            let Some(fraction) = crossing(&points[index], from, to, index == 0, cursor) else {
                return false;
            };
            cursor = fraction;
            self.target = points.get(index + 1).map(|point| point.id);
            if self.target.is_none() {
                return true;
            }
        }
    }
}

impl TaskProgress {
    fn advance(&mut self, points: &[TaskPoint], from: PositionReport, to: PositionReport) {
        if points.len() < 2 {
            return;
        }
        let gap = (to.utc.unix_milliseconds() - from.utc.unix_milliseconds()) as f64;
        let mut cursor = -1.;
        while self.reached < points.len() {
            let start = (self.reached <= 1)
                .then(|| crossing(&points[0], from, to, true, cursor))
                .flatten();
            let entry = (self.reached >= 1)
                .then(|| crossing(&points[self.reached], from, to, false, cursor))
                .flatten();
            let (fraction, starts) = match (start, entry) {
                (Some(start), Some(entry)) if start < entry => (start, true),
                (_, Some(entry)) => (entry, false),
                (Some(start), None) => (start, true),
                (None, None) => return,
            };
            cursor = fraction;
            let time = TaskTime {
                unix_milliseconds: from.utc.unix_milliseconds() + (fraction * gap).round() as i64,
            };
            if starts {
                self.reached = 1;
                self.start = Some(time);
            } else {
                self.reached += 1;
                if self.reached == points.len() {
                    self.finish = Some(time);
                }
            }
        }
    }
}

/// Returns the fraction along the segment where it exits or enters the
/// cylinder of `point`, when the crossing is after `cursor`.
fn crossing(
    point: &TaskPoint,
    from: PositionReport,
    to: PositionReport,
    exit: bool,
    cursor: f64,
) -> Option<f64> {
    let center = point.target.position()?;
    let center = LatLon::from_degrees(center.latitude_degrees, center.longitude_degrees);
    let (entry, leave) = cylinder::crossings(from.position, to.position, center);
    (if exit { leave } else { entry }).filter(|fraction| *fraction > cursor + 1e-9)
}
