import type { Settings } from '$lib/protocol/generated/Settings';

export function defaultSettings(): Settings {
  return {
    locale: null,
    polar: 'LS 8',
    arrivalReserve: 200,
    climbAverageMethod: 'smoothed20s',
    energyCompensation: true,
    flarmPositionCorrection: true,
    hillshadeDirection: 'fixed',
    units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
  };
}
