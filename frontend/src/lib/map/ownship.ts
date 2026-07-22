import type * as GeoJSON from 'geojson';
import type { GnssState } from '$lib/protocol/generated/GnssState';

export function gnssCoordinates(gnss: GnssState): [number, number] {
  return [gnss.position.longitudeDegrees, gnss.position.latitudeDegrees];
}

/** Builds the GeoJSON point feature that positions the ownship symbol. */
export function ownshipFeature(gnss: GnssState): GeoJSON.Feature<GeoJSON.Point> {
  return {
    type: 'Feature',
    geometry: {
      type: 'Point',
      coordinates: gnssCoordinates(gnss),
    },
    properties: {
      track: gnss.trackDegrees ?? 0,
    },
  };
}
