import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';

import { describe, expect, it, vi } from 'vitest';

import { TrafficStore } from './traffic.svelte';

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

describe('TrafficStore', () => {
  it('starts uninitialized without targets', () => {
    let store = new TrafficStore();

    expect(store.initialized).toBe(false);
    expect(store.current).toEqual(new Map());
  });

  it('replaces all targets in the same map on a snapshot', () => {
    let store = new TrafficStore();
    let current = store.current;
    let first = target('flarm:000001');
    let second = target('flarm:000002');

    store.apply({ topic: 'traffic', value: { type: 'snapshot', value: [first] } });

    expect(store.initialized).toBe(true);

    store.apply({ topic: 'traffic', value: { type: 'snapshot', value: [second] } });

    expect(store.current).toBe(current);
    expect(store.current).toEqual(new Map([['flarm:000002', second]]));
  });

  it('inserts a new target on a delta', () => {
    let store = new TrafficStore();
    let first = target('flarm:000001');
    let second = target('flarm:000002');

    store.apply({ topic: 'traffic', value: { type: 'snapshot', value: [first] } });
    store.apply({
      topic: 'traffic',
      value: { type: 'delta', value: { upserts: [second], removed: [] } },
    });

    expect(store.current).toEqual(
      new Map([
        ['flarm:000001', first],
        ['flarm:000002', second],
      ]),
    );
  });

  it('replaces a complete existing target on a delta', () => {
    let store = new TrafficStore();
    let first = target('flarm:000001');
    let replacement = target('flarm:000001', {
      position: { latitudeDegrees: 50.824, longitudeDegrees: 6.187 },
      altitudeMslMeters: null,
      trackDegrees: null,
      stale: true,
    });

    store.apply({ topic: 'traffic', value: { type: 'snapshot', value: [first] } });
    store.apply({
      topic: 'traffic',
      value: { type: 'delta', value: { upserts: [replacement], removed: [] } },
    });

    expect(store.current).toEqual(new Map([['flarm:000001', replacement]]));
  });

  it('applies all delta upserts before removals', () => {
    let store = new TrafficStore();
    let removedAfterUpsert = target('flarm:000001');

    store.apply({
      topic: 'traffic',
      value: {
        type: 'delta',
        value: {
          upserts: [removedAfterUpsert],
          removed: [removedAfterUpsert.id],
        },
      },
    });

    expect(store.current).toEqual(new Map());
  });

  it('ignores non-traffic topics', () => {
    let store = new TrafficStore();
    let initial = store.current;

    store.apply({
      topic: 'settings',
      value: {
        locale: 'de',
        polar: 'LS 8',
        arrivalReserve: 200,
        units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      },
    });

    expect(store.current).toBe(initial);
    expect(store.initialized).toBe(false);
  });

  it('notifies subscribers after applying an update', () => {
    let store = new TrafficStore();
    let subscriber = vi.fn();
    let update = { type: 'snapshot' as const, value: [target('flarm:000001')] };
    store.subscribe(subscriber);

    store.apply({ topic: 'traffic', value: update });

    expect(subscriber).toHaveBeenCalledOnce();
    expect(subscriber).toHaveBeenCalledWith(update, store.current);
    expect(subscriber.mock.calls[0][1]).toBe(store.current);
  });

  it('stops notifications after unsubscribe', () => {
    let store = new TrafficStore();
    let subscriber = vi.fn();
    let unsubscribe = store.subscribe(subscriber);
    unsubscribe();

    store.apply({
      topic: 'traffic',
      value: { type: 'snapshot', value: [target('flarm:000001')] },
    });

    expect(subscriber).not.toHaveBeenCalled();
  });
});
