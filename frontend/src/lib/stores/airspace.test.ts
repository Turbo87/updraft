import { describe, expect, it } from 'vitest';

import { AirspaceStore } from './airspace.svelte';

describe('AirspaceStore', () => {
  it('starts without an airspace source', () => {
    let store = new AirspaceStore();

    expect(store.initialized).toBe(false);
    expect(store.current).toEqual({ generation: 0, sources: [] });
  });

  it('replaces its value with each airspace topic', () => {
    let store = new AirspaceStore();

    store.apply({
      topic: 'airspace',
      value: {
        generation: 1,
        sources: [{ type: 'active', sourceName: 'openair.txt', airspaceCount: 42 }],
      },
    });

    expect(store.initialized).toBe(true);
    expect(store.current).toEqual({
      generation: 1,
      sources: [{ type: 'active', sourceName: 'openair.txt', airspaceCount: 42 }],
    });

    store.apply({
      topic: 'airspace',
      value: {
        generation: 0,
        sources: [{ type: 'unavailable', sourceName: 'openair.txt', error: 'parseFailed' }],
      },
    });

    expect(store.current).toEqual({
      generation: 0,
      sources: [{ type: 'unavailable', sourceName: 'openair.txt', error: 'parseFailed' }],
    });
  });

  it('ignores unrelated topics before initialization', () => {
    let store = new AirspaceStore();

    store.apply({
      topic: 'settings',
      value: {
        locale: 'de',
        polar: 'LS 8',
        arrivalReserve: 200,
        units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      },
    });

    expect(store.initialized).toBe(false);
    expect(store.current).toEqual({ generation: 0, sources: [] });
  });
});
