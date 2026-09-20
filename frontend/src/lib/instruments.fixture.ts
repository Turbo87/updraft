import type { Instruments } from '$lib/protocol/generated/Instruments';

export function instrumentsFixture(overrides: Partial<Instruments> = {}): Instruments {
  return {
    gps: null,
    pressureAltitude: null,
    trueAirspeed: null,
    terrainElevation: null,
    altitudeAgl: null,
    solarPosition: null,
    derived: null,
    ...overrides,
  };
}
