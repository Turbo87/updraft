import { describe, expect, it } from 'vitest';

import { SettingsStore } from './settings.svelte';

describe('SettingsStore', () => {
  it('replaces its value with the latest settings topic', () => {
    let store = new SettingsStore();

    store.apply({
      topic: 'settings',
      value: { locale: 'de' },
    });

    expect(store.current).toEqual({ locale: 'de' });
  });

  it('ignores unrelated topics', () => {
    let store = new SettingsStore();

    store.apply({
      topic: 'instruments',
      value: {
        position: null,
        trackDegrees: null,
        groundSpeedMetersPerSecond: null,
        altitudeMslMeters: null,
      },
    });

    expect(store.current).toEqual({ locale: null });
  });
});
