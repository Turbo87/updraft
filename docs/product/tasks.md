# Ordered tasks

Status: Current behavior

Updraft retains one editable task across restarts. A task is an ordered list of
waypoint snapshots. The first point is the start, the last is the finish, and
intermediate points are turnpoints. Each point uses a 500 m radius cylinder.
A running task requires at least two points.

## Planning and navigation

Open **Navigation** from the menu or the active navigation bar, then open
**Task**. The editor searches the enabled waypoint data. Add points, move them
up or down, or remove them. The map displays the route and each point’s
500 m cylinder with an outline and a light fill. Cylinder display uses a
spherical approximation. Crossing detection uses WGS84 geometry.

Selecting a task point makes it current and selects task guidance as primary
navigation. The pilot can skip ahead or return to an earlier point. A separate
goto does not stop task tracking. Returning primary navigation to the task
follows its current point without resetting running progress.

The task can be pinned. Its pinned guidance follows the current point. Pinning
and unpinning do not start or stop tracking. The navigation selection screen
shows the task once: as the current target, otherwise as a pin, otherwise as
its dedicated entry. The task never enters recent goto history.

Live edits retain the current point's identity. Removing it selects the following
point, or the preceding point if none follows. The first and last roles follow
the new order. Edits do not count as crossings. They reset crossing detection
until a fresh position establishes the new baseline. The editor rejects removing
a point when that would leave a running task with fewer than two points.

## Crossings and restarts

An exit from the start cylinder records a start and selects the second point.
Entry into an intermediate cylinder selects the following point. Entry into the
finish completes the task. The detector uses only the fixes that the flight
recorder records, in order. A fix without UTC, or a fix that the recorder drops,
does not count. The detector checks the geodesic segment between two
consecutive recorded fixes, including a complete cylinder passage between them.
A segment has no maximum length, and a source change does not interrupt it. It
uses WGS84 geometry. A numerical tolerance of one micrometer applies when a
reported position lies on a cylinder boundary.

Task edits, manual point selection, and restoration establish a new baseline.
Selecting a point while inside its cylinder requires leaving and re-entering.
Crossings before that point became current do not count for it.

The start remains monitored after the first start exit. Another exit replaces
the start time until the pilot reaches the second point or manually selects a
point beyond it. Selecting the start in task details reopens this window. A
standalone goto to the same waypoint does not affect it. Skipping the start does
not invent a start time.

Crossing times interpolate along the segment from the UTC of its fixes. The
details page displays the start and finish times in UTC.

## Stopping, completion, and storage

**Stop task** pauses tracking and retains the route, current point, and recorded
times. It clears primary navigation only when that follows the task. Selecting a
task point or using the task navigation arrow resumes tracking.

Completion clears primary navigation only when it follows the task. A separate
goto remains unchanged. Selecting a point after completion resumes tracking,
clears the finish time, and retains the start time. Selecting the start also
reopens the restart window.

The core owns the task, guidance, and crossing state. The shell saves route and
progress changes, including automatic changes while another target is primary.
Restoration retains running, stopped, or completed status. It does not infer
crossings during the interruption. A failed save retains live state and exposes
a retry action that saves the current state without repeating an edit.

Competition rules, additional observation zones, task import, a named task
library, and record validation require later slices. Physical Android lifecycle
and in-flight readability checks remain open.
