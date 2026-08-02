import { describe, expect, it } from 'vitest';

import { AirspaceStore } from './airspace.svelte';

describe('AirspaceStore', () => {
  it('starts without an airspace source', () => {
    let store = new AirspaceStore();

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
});
