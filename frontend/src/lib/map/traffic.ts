import type * as GeoJSON from 'geojson';
import type { GeoJSONSourceDiff } from 'maplibre-gl';
import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';
import type { TrafficAlarmLevel } from '$lib/protocol/generated/TrafficAlarmLevel';
import type { TrafficDelta } from '$lib/protocol/generated/TrafficDelta';
import type { TrafficType } from '$lib/protocol/generated/TrafficType';

export type TrafficFeatureProperties = {
  trafficType: TrafficType;
  alarmLevel: TrafficAlarmLevel;
  stale: boolean;
  trackDegrees: number | null;
  altitudeMslMeters: number | null;
};

export function trafficFeature(
  target: PublishedTrafficTarget,
): GeoJSON.Feature<GeoJSON.Point, TrafficFeatureProperties> {
  return {
    type: 'Feature',
    id: target.id,
    geometry: {
      type: 'Point',
      coordinates: [target.position.longitudeDegrees, target.position.latitudeDegrees],
    },
    properties: {
      trafficType: target.trafficType,
      alarmLevel: target.alarmLevel,
      stale: target.stale,
      trackDegrees: target.trackDegrees,
      altitudeMslMeters: target.altitudeMslMeters,
    },
  };
}

export function trafficFeatureCollection(
  targets: Iterable<PublishedTrafficTarget>,
): GeoJSON.FeatureCollection<GeoJSON.Point, TrafficFeatureProperties> {
  return {
    type: 'FeatureCollection',
    features: Array.from(targets, trafficFeature),
  };
}

export function trafficSourceDiff(delta: TrafficDelta): GeoJSONSourceDiff {
  return {
    ...(delta.removed.length > 0 && { remove: delta.removed }),
    ...(delta.upserts.length > 0 && { add: delta.upserts.map(trafficFeature) }),
  };
}
