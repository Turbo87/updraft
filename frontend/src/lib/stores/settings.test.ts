import { describe, expect, it } from 'vitest';

import { instrumentsFixture } from '$lib/instruments.fixture';
import { SettingsStore } from './settings.svelte';

describe('SettingsStore', () => {
  it('creates independent initial settings for each store', () => {
    let first = new SettingsStore();
    let second = new SettingsStore();

    expect(first.current).toEqual(second.current);
    expect(first.current).not.toBe(second.current);
    expect(first.current.units).not.toBe(second.current.units);
  });

  it('replaces its value with the latest settings topic', () => {
    let store = new SettingsStore();

    store.apply({
      topic: 'settings',
      value: {
        locale: 'de',
        polar: 'LS 8-18',
        arrivalReserve: 200,
        climbAverageMethod: 'normalizedEma',
        hillshadeDirection: 'wind',
        energyCompensation: true,
        flarmPositionCorrection: true,
        units: { altitude: 'ft', distance: 'nm', speed: 'kt', verticalSpeed: 'ft/min' },
      },
    });

    expect(store.current).toEqual({
      locale: 'de',
      polar: 'LS 8-18',
      arrivalReserve: 200,
      climbAverageMethod: 'normalizedEma',
      hillshadeDirection: 'wind',
      energyCompensation: true,
      flarmPositionCorrection: true,
      units: { altitude: 'ft', distance: 'nm', speed: 'kt', verticalSpeed: 'ft/min' },
    });
  });

  it('ignores unrelated topics', () => {
    let store = new SettingsStore();

    store.apply({
      topic: 'instruments',
      value: instrumentsFixture(),
    });

    expect(store.current).toEqual({
      locale: null,
      polar: 'LS 8',
      arrivalReserve: 200,
      climbAverageMethod: 'smoothed20s',
      hillshadeDirection: 'fixed',
      energyCompensation: true,
      flarmPositionCorrection: true,
      units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
    });
  });
});
