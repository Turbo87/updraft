import type { Navigation } from '$lib/protocol/generated/Navigation';

export function waypointNavigation(name: string) {
  return {
    target: {
      type: 'waypoint',
      name,
      latitudeDegrees: 50,
      longitudeDegrees: 6,
      elevationMeters: 100,
    },
    position: { latitudeDegrees: 50, longitudeDegrees: 6 },
    traffic: null,
    arrival: { marginMeters: 250, stale: false },
    guidance: {
      distanceMeters: 12300,
      bearingDegrees: 90,
      relativeBearingDegrees: -15,
      stale: false,
    },
  } satisfies Navigation;
}
