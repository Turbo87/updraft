import type { LatLon } from '$lib/protocol/generated/LatLon';

import { bearing as turfBearing } from '@turf/bearing';
import { distance as turfDistance } from '@turf/distance';

/** Calculates geographic distance in meters and initial true bearing in degrees. */
export function calculateDistanceAndBearing(
  from: LatLon,
  to: LatLon,
): { distanceMeters: number; bearingDegrees: number } {
  let fromCoordinate: [number, number] = [from.longitudeDegrees, from.latitudeDegrees];
  let toCoordinate: [number, number] = [to.longitudeDegrees, to.latitudeDegrees];
  let bearingDegrees = turfBearing(fromCoordinate, toCoordinate);

  return {
    distanceMeters: turfDistance(fromCoordinate, toCoordinate, { units: 'meters' }),
    bearingDegrees: (bearingDegrees + 360) % 360,
  };
}
