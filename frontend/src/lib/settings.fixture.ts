import type { Settings } from '$lib/protocol/generated/Settings';

export function settingsFixture(overrides: Partial<Settings> = {}): Settings {
  return {
    locale: null,
    polar: 'LS 8',
    arrivalReserve: 200,
    climbAverageMethod: 'smoothed20s',
    hillshadeDirection: 'fixed',
    energyCompensation: true,
    flarmPositionCorrection: true,
    ...overrides,
    units: {
      altitude: 'm',
      distance: 'km',
      speed: 'km/h',
      verticalSpeed: 'm/s',
      ...overrides.units,
    },
  };
}
