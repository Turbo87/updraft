import { describe, expect, it } from 'vitest';

import { InstrumentsStore } from './instruments.svelte';

describe('InstrumentsStore', () => {
  it('replaces its value with the latest topic', () => {
    let store = new InstrumentsStore();

    store.apply({
      topic: 'instruments',
      value: {
        position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
        trackDegrees: 270,
        groundSpeedMetersPerSecond: 45,
        altitudeMslMeters: 200,
      },
    });

    expect(store.current.trackDegrees).toBe(270);
    expect(store.current.position).toEqual({
      latitudeDegrees: 50.823,
      longitudeDegrees: 6.186,
    });
  });
});
