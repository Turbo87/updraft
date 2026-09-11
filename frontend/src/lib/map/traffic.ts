import type * as GeoJSON from 'geojson';
import type { ErrorEvent, GeoJSONSource, GeoJSONSourceDiff, Subscription } from 'maplibre-gl';
import type { AltitudeUnit } from '$lib/protocol/generated/AltitudeUnit';
import type { ClimbAverageMethod } from '$lib/protocol/generated/ClimbAverageMethod';
import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';
import type { TrafficAlarmLevel } from '$lib/protocol/generated/TrafficAlarmLevel';
import type { TrafficDelta } from '$lib/protocol/generated/TrafficDelta';
import type { TrafficType } from '$lib/protocol/generated/TrafficType';
import type { TrafficUpdate } from '$lib/protocol/generated/TrafficUpdate';
import type { VerticalSpeedUnit } from '$lib/protocol/generated/VerticalSpeedUnit';

import { convertAltitude, convertVerticalSpeed } from '$lib/units';

type TrafficGeoJSONSource = Pick<GeoJSONSource, 'setData' | 'updateData'> & {
  on(type: 'error', listener: (event: ErrorEvent) => void): Subscription;
};

export type TrafficFeatureProperties = {
  id: string;
  trafficType: TrafficType;
  alarmLevel: TrafficAlarmLevel;
  stale: boolean;
  trackDegrees: number | null;
  label: string | null;
};

export function trafficFeature(
  target: PublishedTrafficTarget,
  altitudeUnit: AltitudeUnit,
  verticalSpeedUnit: VerticalSpeedUnit,
  method: ClimbAverageMethod = 'smoothed20s',
): GeoJSON.Feature<GeoJSON.Point, TrafficFeatureProperties> {
  let altitude = target.altitudeMslMeters;
  let altitudeLabel =
    altitude === null
      ? null
      : `${Math.round(convertAltitude(altitude, altitudeUnit))} ${altitudeUnit}`;
  let name = target.flarmnet?.callSign || target.flarmnet?.registration;
  let climb = target.climb?.[method];
  let climbValue =
    !target.stale && climb !== undefined
      ? convertVerticalSpeed(climb, verticalSpeedUnit).toFixed(
          verticalSpeedUnit === 'ft/min' ? 0 : 1,
        )
      : null;
  let climbLabel =
    climbValue !== null && Number(climbValue) > 0 ? `+${climbValue} ${verticalSpeedUnit}` : null;
  let label = [name, altitudeLabel, climbLabel].filter(Boolean).join('\n') || null;

  return {
    type: 'Feature',
    id: target.id,
    geometry: {
      type: 'Point',
      coordinates: [target.position.longitudeDegrees, target.position.latitudeDegrees],
    },
    properties: {
      id: target.id,
      trafficType: target.trafficType,
      alarmLevel: target.alarmLevel,
      stale: target.stale,
      trackDegrees: target.trackDegrees,
      label,
    },
  };
}

export function trafficFeatureCollection(
  targets: Iterable<PublishedTrafficTarget>,
  altitudeUnit: AltitudeUnit,
  verticalSpeedUnit: VerticalSpeedUnit,
  method: ClimbAverageMethod = 'smoothed20s',
): GeoJSON.FeatureCollection<GeoJSON.Point, TrafficFeatureProperties> {
  return {
    type: 'FeatureCollection',
    features: Array.from(targets, (target) =>
      trafficFeature(target, altitudeUnit, verticalSpeedUnit, method),
    ),
  };
}

export function trafficSourceDiff(
  delta: TrafficDelta,
  altitudeUnit: AltitudeUnit,
  verticalSpeedUnit: VerticalSpeedUnit,
  method: ClimbAverageMethod = 'smoothed20s',
): GeoJSONSourceDiff {
  return {
    ...(delta.removed.length > 0 && { remove: delta.removed }),
    ...(delta.upserts.length > 0 && {
      add: delta.upserts.map((target) =>
        trafficFeature(target, altitudeUnit, verticalSpeedUnit, method),
      ),
    }),
  };
}

export async function applyTrafficSourceUpdate(
  source: TrafficGeoJSONSource,
  update: TrafficUpdate,
  currentTargets: ReadonlyMap<string, PublishedTrafficTarget>,
  altitudeUnit: AltitudeUnit,
  verticalSpeedUnit: VerticalSpeedUnit,
  method: ClimbAverageMethod = 'smoothed20s',
): Promise<void> {
  if (update.type === 'snapshot') {
    await source.setData(
      trafficFeatureCollection(currentTargets.values(), altitudeUnit, verticalSpeedUnit, method),
    );
    return;
  }

  let sourceError: unknown;
  let errorSubscription = source.on('error', (event) => {
    sourceError ??= event.error;
  });

  try {
    await source.updateData(
      trafficSourceDiff(update.value, altitudeUnit, verticalSpeedUnit, method),
    );
  } catch (error) {
    sourceError ??= error;
  } finally {
    errorSubscription.unsubscribe();
  }

  if (!sourceError) return;

  console.warn('Traffic source update failed. Rebuilding the source.', sourceError);
  await source.setData(
    trafficFeatureCollection(currentTargets.values(), altitudeUnit, verticalSpeedUnit, method),
  );
}
