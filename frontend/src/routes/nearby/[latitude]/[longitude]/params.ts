import type { LatLon } from '$lib/protocol/generated/LatLon';

/** Parses finite nearby route coordinates within the inclusive geographic bounds. */
export function parseNearbyRouteCoordinates(
  latitude: string | undefined,
  longitude: string | undefined,
): LatLon | null {
  let latitudeDegrees = parseDecimalCoordinate(latitude);
  let longitudeDegrees = parseDecimalCoordinate(longitude);

  if (
    latitudeDegrees === null ||
    latitudeDegrees < -90 ||
    latitudeDegrees > 90 ||
    longitudeDegrees === null ||
    longitudeDegrees < -180 ||
    longitudeDegrees > 180
  ) {
    return null;
  }

  return { latitudeDegrees, longitudeDegrees };
}

function parseDecimalCoordinate(value: string | undefined): number | null {
  if (value === undefined || value.trim() === '') return null;

  let parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}
