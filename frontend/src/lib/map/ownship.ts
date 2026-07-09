import type * as GeoJSON from 'geojson';
import type { OwnshipPosition } from '$lib/protocol/generated/OwnshipPosition';

/** Builds the GeoJSON point feature that positions the ownship symbol. */
export function ownshipFeature(position: OwnshipPosition): GeoJSON.Feature<GeoJSON.Point> {
  return {
    type: 'Feature',
    geometry: {
      type: 'Point',
      coordinates: [position.location.longitude, position.location.latitude],
    },
    properties: {
      track: position.track ?? 0,
    },
  };
}
