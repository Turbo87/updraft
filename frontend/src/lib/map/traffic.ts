import type * as GeoJSON from 'geojson';
import type { ErrorEvent, GeoJSONSource, GeoJSONSourceDiff, Subscription } from 'maplibre-gl';
import type { AltitudeUnit } from '$lib/protocol/generated/AltitudeUnit';
import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';
import type { TrafficAlarmLevel } from '$lib/protocol/generated/TrafficAlarmLevel';
import type { TrafficDelta } from '$lib/protocol/generated/TrafficDelta';
import type { TrafficType } from '$lib/protocol/generated/TrafficType';
import type { TrafficUpdate } from '$lib/protocol/generated/TrafficUpdate';

import { convertAltitude } from '$lib/units';

type TrafficGeoJSONSource = Pick<GeoJSONSource, 'setData' | 'updateData'> & {
  on(type: 'error', listener: (event: ErrorEvent) => void): Subscription;
};

export type TrafficFeatureProperties = {
  trafficType: TrafficType;
  alarmLevel: TrafficAlarmLevel;
  stale: boolean;
  trackDegrees: number | null;
  altitudeLabel: string | null;
};

export function trafficFeature(
  target: PublishedTrafficTarget,
  altitudeUnit: AltitudeUnit,
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
      altitudeLabel:
        target.altitudeMslMeters === null
          ? null
          : `${Math.round(convertAltitude(target.altitudeMslMeters, altitudeUnit))} ${altitudeUnit}`,
    },
  };
}

export function trafficFeatureCollection(
  targets: Iterable<PublishedTrafficTarget>,
  altitudeUnit: AltitudeUnit,
): GeoJSON.FeatureCollection<GeoJSON.Point, TrafficFeatureProperties> {
  return {
    type: 'FeatureCollection',
    features: Array.from(targets, (target) => trafficFeature(target, altitudeUnit)),
  };
}

export function trafficSourceDiff(
  delta: TrafficDelta,
  altitudeUnit: AltitudeUnit,
): GeoJSONSourceDiff {
  return {
    ...(delta.removed.length > 0 && { remove: delta.removed }),
    ...(delta.upserts.length > 0 && {
      add: delta.upserts.map((target) => trafficFeature(target, altitudeUnit)),
    }),
  };
}

export async function applyTrafficSourceUpdate(
  source: TrafficGeoJSONSource,
  update: TrafficUpdate,
  currentTargets: ReadonlyMap<string, PublishedTrafficTarget>,
  altitudeUnit: AltitudeUnit,
): Promise<void> {
  if (update.type === 'snapshot') {
    await source.setData(trafficFeatureCollection(currentTargets.values(), altitudeUnit));
    return;
  }

  let sourceError: unknown;
  let errorSubscription = source.on('error', (event) => {
    sourceError ??= event.error;
  });

  try {
    await source.updateData(trafficSourceDiff(update.value, altitudeUnit));
  } catch (error) {
    sourceError ??= error;
  } finally {
    errorSubscription.unsubscribe();
  }

  if (!sourceError) return;

  console.warn('Traffic source update failed. Rebuilding the source.', sourceError);
  await source.setData(trafficFeatureCollection(currentTargets.values(), altitudeUnit));
}
