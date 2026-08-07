import { describe, expect, it } from 'vitest';

import { AirspaceStore } from './airspace.svelte';

describe('AirspaceStore', () => {
  it('starts without an airspace source', () => {
    let store = new AirspaceStore();

    expect(store.initialized).toBe(false);
    expect(store.current).toEqual({ type: 'none' });
  });

  it('replaces its value with each airspace topic', () => {
    let store = new AirspaceStore();

    store.apply({
      topic: 'airspace',
      value: {
        type: 'active',
        sourceName: 'openair.txt',
        airspaceCount: 42,
        generation: 1,
      },
    });

    expect(store.initialized).toBe(true);
    expect(store.current).toEqual({
      type: 'active',
      sourceName: 'openair.txt',
      airspaceCount: 42,
      generation: 1,
    });

    store.apply({
      topic: 'airspace',
      value: {
        type: 'unavailable',
        sourceName: 'openair.txt',
        error: 'parseFailed',
      },
    });

    expect(store.current).toEqual({
      type: 'unavailable',
      sourceName: 'openair.txt',
      error: 'parseFailed',
    });
  });

  it('ignores unrelated topics before initialization', () => {
    let store = new AirspaceStore();

    store.apply({
      topic: 'settings',
      value: {
        locale: 'de',
        units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      },
    });

    expect(store.initialized).toBe(false);
    expect(store.current).toEqual({ type: 'none' });
  });
});
