import type { NavigationTarget } from '$lib/protocol/generated/NavigationTarget';
import type { WaypointFeature } from '$lib/waypoints';

export function targetsMatch(a: NavigationTarget, b: NavigationTarget): boolean {
  if (a.type === 'task' || b.type === 'task') return a.type === 'task' && b.type === 'task';
  if (a.type === 'traffic' || b.type === 'traffic')
    return a.type === 'traffic' && b.type === 'traffic' && a.id === b.id;
  if (a.type !== b.type) return false;
  if (a.type === 'waypoint' && b.type === 'waypoint' && a.name !== b.name) return false;
  let longitude = Math.abs(a.longitudeDegrees - b.longitudeDegrees);
  return (
    Math.abs(a.latitudeDegrees - b.latitudeDegrees) <= 0.0001 &&
    Math.min(longitude, 360 - longitude) <= 0.0001
  );
}

export function waypointTarget(waypoint: WaypointFeature): NavigationTarget {
  return {
    type: 'waypoint',
    name: waypoint.properties.name,
    latitudeDegrees: waypoint.geometry.coordinates[1],
    longitudeDegrees: waypoint.geometry.coordinates[0],
    elevationMeters: waypoint.properties.elevationMeters,
  };
}
