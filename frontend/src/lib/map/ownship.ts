import type * as GeoJSON from 'geojson';

export interface OwnshipPosition {
  longitude: number;
  latitude: number;
  track?: number;
}

/** Builds the GeoJSON point feature that positions the ownship symbol. */
export function ownshipFeature(position: OwnshipPosition): GeoJSON.Feature<GeoJSON.Point> {
  return {
    type: 'Feature',
    geometry: {
      type: 'Point',
      coordinates: [position.longitude, position.latitude],
    },
    properties: {
      track: position.track ?? 0,
    },
  };
}
