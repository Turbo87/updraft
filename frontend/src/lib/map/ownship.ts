import type * as GeoJSON from 'geojson';
import type { PositionFix } from '$lib/protocol/generated/PositionFix';

export function positionCoordinates(position: PositionFix): [number, number] {
  return [position.longitudeDegrees, position.latitudeDegrees];
}

/** Builds the GeoJSON point feature that positions the ownship symbol. */
export function ownshipFeature(position: PositionFix): GeoJSON.Feature<GeoJSON.Point> {
  return {
    type: 'Feature',
    geometry: {
      type: 'Point',
      coordinates: positionCoordinates(position),
    },
    properties: {
      track: position.trackDegrees ?? 0,
    },
  };
}
