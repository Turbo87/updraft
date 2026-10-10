import type { Trail } from '#lib/protocol/generated/Trail.js';
import type { TrailSubscriber } from './trail';

import { describe, expect, it, vi } from 'vitest';

import { TrailStore } from './trail';

function trail(unixMilliseconds: number): Trail {
  return {
    recordingStart: 0,
    sample: {
      unixMilliseconds,
      position: { latitudeDegrees: 50, longitudeDegrees: 6 },
      altitudeMslMeters: null,
      altitudeAglMeters: null,
      varioMetersPerSecond: null,
      nettoMetersPerSecond: null,
      relativeVarioMetersPerSecond: null,
    },
  };
}

describe('TrailStore', () => {
  it('sends each source change to its subscribers', () => {
    let store = new TrailStore();
    let subscriber = vi.fn<TrailSubscriber>();
    let unsubscribe = store.subscribe(subscriber);

    store.apply({ topic: 'trail', value: trail(0) });
    store.apply({ topic: 'taskSaveFailed', value: false });
    store.apply({ topic: 'trail', value: trail(1_000) });
    unsubscribe();
    store.apply({ topic: 'trail', value: null });

    expect(subscriber.mock.calls.map(([diff]) => diff.add?.map(({ id }) => id))).toEqual([[1_000]]);
  });
});
