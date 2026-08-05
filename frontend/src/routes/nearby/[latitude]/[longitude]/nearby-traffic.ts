import type { Locale } from '$lib/protocol/generated/Locale';
import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';
import type { TrafficAlarmLevel } from '$lib/protocol/generated/TrafficAlarmLevel';
import type { TrafficType } from '$lib/protocol/generated/TrafficType';

import { m } from '$lib/paraglide/messages.js';

export type RetainedTraffic = {
  id: string;
  target: PublishedTrafficTarget | null;
  available: boolean;
};

export function createRetainedTraffic(
  ids: string[],
  currentTargets: ReadonlyMap<string, PublishedTrafficTarget>,
): RetainedTraffic[] {
  return ids.map((id) => {
    let target = currentTargets.get(id) ?? null;
    return { id, target, available: target !== null };
  });
}

export function refreshRetainedTraffic(
  retainedTraffic: RetainedTraffic[],
  currentTargets: ReadonlyMap<string, PublishedTrafficTarget>,
): RetainedTraffic[] {
  return retainedTraffic.map((retained) => {
    let target = currentTargets.get(retained.id);
    return target
      ? { id: retained.id, target, available: true }
      : { ...retained, available: false };
  });
}

export function formatTrafficId(id: string): string {
  let separator = id.indexOf(':');
  return separator === -1
    ? id
    : `${id.slice(0, separator).toUpperCase()} ${id.slice(separator + 1)}`;
}

export function formatTrafficType(trafficType: TrafficType, locale: Locale): string {
  return m.traffic_type_value({ trafficType }, { locale });
}

export function formatTrafficAlarmLevel(alarmLevel: TrafficAlarmLevel, locale: Locale): string {
  return m.traffic_alarm_level_value({ alarmLevel }, { locale });
}
