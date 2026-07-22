import type * as GeoJSON from 'geojson';
import type { Availability } from '$lib/protocol/generated/Availability';
import type { LatLon } from '$lib/protocol/generated/LatLon';

export function latLonCoordinates(position: LatLon): [number, number] {
  return [position.longitudeDegrees, position.latitudeDegrees];
}

/** Builds the GeoJSON point feature that positions the ownship symbol. */
export function ownshipFeature(
  position: LatLon,
  track: Availability<number>,
): GeoJSON.Feature<GeoJSON.Point> {
  return {
    type: 'Feature',
    geometry: {
      type: 'Point',
      coordinates: latLonCoordinates(position),
    },
    properties: {
      track: track.status === 'current' ? track.value : 0,
    },
  };
}
