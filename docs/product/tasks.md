# Ordered tasks

Status: Current behavior

Updraft retains one editable task across restarts. A task is an ordered list of
waypoint snapshots. The first point is the start, the last is the finish, and
intermediate points are turnpoints. Each point uses a 500 m radius cylinder.
A task with fewer than two points has no progress and no navigation target.

## Planning and navigation

Open **Navigation** from the menu or the active navigation bar, then open
**Task**. The editor searches the enabled waypoint data. Add points, move them
up or down, or remove them. The map displays the route and each point’s
500 m cylinder with an outline and a light fill. Cylinder display uses a
spherical approximation. Crossing detection uses WGS84 geometry.

Selecting a task point makes it the navigation target of the task and selects
task guidance as primary navigation. The pilot can skip ahead or return to an
earlier point. A separate goto does not change the task. Returning primary
navigation to the task follows its navigation target.

The task can be pinned. Its pinned guidance follows the navigation target.
The navigation selection screen shows the task once: as the current target,
otherwise as a pin, otherwise as its dedicated entry. The task never enters
recent goto history.

## Progress

Task progress is the set of reached task points and the start and finish times.
It is a function of the route and the fixes that the flight recorder records.
The navigation target and manual selection do not change it.

Progress uses only the recorded fixes, in order. A fix without UTC, or a fix
that the recorder drops, does not count. A new recording resets progress and
keeps the navigation target. The detector checks the geodesic segment between
two consecutive recorded fixes, including a complete cylinder passage between
them. A segment has no maximum length, and a source change does not interrupt
it. A numerical tolerance of one micrometer applies when a reported position
lies on a cylinder boundary.

The start is the last exit from the start cylinder before the pilot reaches the
second point. Later start exits do not count. For a two-point task, the second
point is the finish. Turnpoints count in route order. An entry counts for a
point only when the start and all earlier points are reached. The finish is the
first entry into the finish cylinder after the last turnpoint is reached.
Nothing changes after the finish.

Crossing times interpolate along the segment from the UTC of its fixes. The
details page displays the start and finish times in UTC.

Each route change derives progress again from all recorded fixes of the current
recording. These include the samples that the core restored at startup. Restarts
after the second point and competition start gates wait for competition rules.

## Navigation target

After startup or a route change, the navigation target is the first unreached
point. With nothing reached, that is the start. While the start is the target, a
start exit moves the target to the second point. Entry into the cylinder of the
target moves the target to the following point, also when the entry does not
count for progress. For example, the pilot skips the second point and selects
the third point. Entry into the third point moves the target to the fourth
point, and the second point stays unreached.

Entry into the finish cylinder while the finish is the target removes the
target. This clears primary navigation only when primary navigation follows the
task. A separate goto remains unchanged. A manual selection holds until one of
these rules moves the target. Selecting a point while inside its cylinder
requires leaving and re-entering.

## Stopping and storage

**Stop task** asks for a confirmation. It then clears the route and the
progress. It clears primary navigation only when that follows the task.

The core owns the route, the progress, and the navigation target. The task file
stores only the route. Task files with saved progress from earlier versions load
as their route. After a restart, the core derives progress from the restored
flight recording. A failed save retains live state and exposes a retry action
that saves the current route without repeating an edit.

Competition rules, additional observation zones, task import, a named task
library, and record validation require later slices. Physical Android lifecycle
and in-flight readability checks remain open.
