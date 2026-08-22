import { describe, expect, it } from 'vitest';

import { InstrumentsStore } from './instruments.svelte';

describe('InstrumentsStore', () => {
  it('replaces its value with the latest topic', () => {
    let store = new InstrumentsStore();

    store.apply({
      topic: 'instruments',
      value: {
        gps: {
          position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
          altitudeMeters: 200,
          groundSpeedMetersPerSecond: 45,
          trackDegrees: 270,
          fixTime: null,
          stale: false,
        },
        pressureAltitude: null,
        trueAirspeed: null,
        derived: null,
      },
    });

    expect(store.current.gps?.trackDegrees).toBe(270);
    expect(store.current.gps?.position).toEqual({
      latitudeDegrees: 50.823,
      longitudeDegrees: 6.186,
    });
  });
});
