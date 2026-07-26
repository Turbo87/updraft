import type * as GeoJSON from 'geojson';
import type { LatLon } from '$lib/protocol/generated/LatLon';

export function positionCoordinates(position: LatLon): [number, number] {
  return [position.longitudeDegrees, position.latitudeDegrees];
}

/** Builds the GeoJSON point feature that positions the ownship symbol. */
export function ownshipFeature(
  position: LatLon,
  trackDegrees: number | null,
): GeoJSON.Feature<GeoJSON.Point> {
  return {
    type: 'Feature',
    geometry: {
      type: 'Point',
      coordinates: positionCoordinates(position),
    },
    properties: {
      track: trackDegrees ?? 0,
    },
  };
}
