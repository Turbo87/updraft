import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';

import { expect, it } from 'vitest';

import {
  createRetainedTraffic,
  formatTrafficAlarmLevel,
  formatTrafficId,
  formatTrafficType,
  refreshRetainedTraffic,
} from './nearby-traffic';

function target(
  id: string,
  overrides: Partial<PublishedTrafficTarget> = {},
): PublishedTrafficTarget {
  return {
    id,
    position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
    altitudeMslMeters: 200,
    trafficType: 'glider',
    trackDegrees: 270,
    alarmLevel: 'none',
    stale: false,
    ...overrides,
  };
}

it('keeps the selected traffic sequence, duplicates, and missing IDs', () => {
  let first = target('flarm:000001');
  let second = target('flarm:000002');
  let current = new Map([
    [first.id, first],
    [second.id, second],
  ]);

  let retained = createRetainedTraffic(
    ['flarm:000002', 'flarm:000001', 'flarm:000002', 'flarm:000003'],
    current,
  );

  expect(
    retained.map(({ id, target, available }) => ({ id, targetId: target?.id ?? null, available })),
  ).toEqual([
    { id: 'flarm:000002', targetId: 'flarm:000002', available: true },
    { id: 'flarm:000001', targetId: 'flarm:000001', available: true },
    { id: 'flarm:000002', targetId: 'flarm:000002', available: true },
    { id: 'flarm:000003', targetId: null, available: false },
  ]);
});

it('updates, retains, and recovers selected traffic without adding targets', () => {
  let first = target('flarm:000001');
  let second = target('flarm:000002');
  let retained = createRetainedTraffic(
    [first.id, second.id],
    new Map([
      [first.id, first],
      [second.id, second],
    ]),
  );
  let updated = target(first.id, { altitudeMslMeters: 300 });
  let unrelated = target('flarm:000003');

  retained = refreshRetainedTraffic(
    retained,
    new Map([
      [updated.id, updated],
      [unrelated.id, unrelated],
    ]),
  );

  expect(
    retained.map(({ id, target, available }) => [id, target?.altitudeMslMeters, available]),
  ).toEqual([
    ['flarm:000001', 300, true],
    ['flarm:000002', 200, false],
  ]);

  let recovered = target(second.id, { altitudeMslMeters: 400 });
  retained = refreshRetainedTraffic(
    retained,
    new Map([
      [updated.id, updated],
      [recovered.id, recovered],
      [unrelated.id, unrelated],
    ]),
  );

  expect(
    retained.map(({ id, target, available }) => [id, target?.altitudeMslMeters, available]),
  ).toEqual([
    ['flarm:000001', 300, true],
    ['flarm:000002', 400, true],
  ]);
});

it('formats a canonical traffic ID for display', () => {
  expect(formatTrafficId('flarm:ABC123')).toBe('FLARM ABC123');
  expect(formatTrafficId('unknown')).toBe('unknown');
});

it('formats a traffic type for the selected locale', () => {
  expect(formatTrafficType('towPlane', 'en')).toBe('Tow plane');
  expect(formatTrafficType('towPlane', 'de')).toBe('Schleppflugzeug');
});

it('formats a traffic alarm level for the selected locale', () => {
  expect(formatTrafficAlarmLevel('important', 'en')).toBe('Important');
  expect(formatTrafficAlarmLevel('important', 'de')).toBe('Wichtig');
});
