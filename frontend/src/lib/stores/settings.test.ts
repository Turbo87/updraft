import { describe, expect, it } from 'vitest';

import { SettingsStore } from './settings.svelte';

describe('SettingsStore', () => {
  it('replaces its value with the latest settings topic', () => {
    let store = new SettingsStore();

    store.apply({
      topic: 'settings',
      value: {
        locale: 'de',
        units: { altitude: 'ft', distance: 'nm', speed: 'kt', verticalSpeed: 'ft/min' },
      },
    });

    expect(store.current).toEqual({
      locale: 'de',
      units: { altitude: 'ft', distance: 'nm', speed: 'kt', verticalSpeed: 'ft/min' },
    });
  });

  it('ignores unrelated topics', () => {
    let store = new SettingsStore();

    store.apply({
      topic: 'instruments',
      value: {
        gps: null,
        pressureAltitude: null,
        trueAirspeed: null,
      },
    });

    expect(store.current).toEqual({
      locale: null,
      units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
    });
  });
});
