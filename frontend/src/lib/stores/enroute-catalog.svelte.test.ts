import type { EnrouteCatalogStatus } from '$lib/client';

import { flushSync } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';

import { BasemapsStore } from './basemaps.svelte';
import { EnrouteCatalogStore } from './enroute-catalog.svelte';

const catalog: EnrouteCatalogStatus = {
  cached: { entries: [], checkedAt: 1000 },
  refreshing: false,
  error: false,
};
let dispose: (() => void) | undefined;
afterEach(() => dispose?.());

function setup() {
  let store = new EnrouteCatalogStore();
  let basemaps = new BasemapsStore();
  let client = { getEnrouteBasemapUpdates: vi.fn<() => Promise<string[]>>() };
  dispose = store.watchBasemapUpdates(client, basemaps);
  return { store, basemaps, client };
}

it('checks at startup and after inventory changes, including disabled files', async () => {
  let { store, basemaps, client } = setup();
  client.getEnrouteBasemapUpdates.mockResolvedValue(['Europe/Germany.mbtiles']);
  store.current = catalog;
  flushSync();
  expect(client.getEnrouteBasemapUpdates).not.toHaveBeenCalled();
  basemaps.current = {
    generation: 1,
    sources: [{ sourceName: 'enroute/Europe/Germany.mbtiles', type: 'disabled' }],
  };
  flushSync();
  await vi.waitFor(() => expect(store.basemapUpdates).toEqual(['Europe/Germany.mbtiles']));
  expect(store.basemapUpdateError).toBe(false);
  client.getEnrouteBasemapUpdates.mockResolvedValue([]);
  basemaps.current = { generation: 2, sources: [] };
  flushSync();
  await vi.waitFor(() => expect(store.basemapUpdates).toEqual([]));
  expect(client.getEnrouteBasemapUpdates).toHaveBeenCalledTimes(2);
});

it('distinguishes failed checks from no updates and recovers after retry', async () => {
  let { store, basemaps, client } = setup();
  basemaps.current = { generation: 1, sources: [] };
  store.current = catalog;
  client.getEnrouteBasemapUpdates.mockRejectedValue(new Error('metadata failed'));
  flushSync();
  await vi.waitFor(() => expect(store.basemapUpdateError).toBe(true));
  expect(store.basemapUpdates).toBeNull();
  client.getEnrouteBasemapUpdates.mockResolvedValue([]);
  store.current = { ...catalog, refreshing: true };
  flushSync();
  expect(client.getEnrouteBasemapUpdates).toHaveBeenCalledTimes(1);
  store.current = { ...catalog };
  flushSync();
  await vi.waitFor(() => expect(store.basemapUpdates).toEqual([]));
  expect(store.basemapUpdateError).toBe(false);
  store.current = { ...catalog, error: true };
  flushSync();
  await vi.waitFor(() => expect(store.basemapUpdates).toEqual([]));
  expect(store.basemapUpdateError).toBe(true);
  expect(store.current.cached?.checkedAt).toBe(1000);
});

it('ignores obsolete results and results after disposal', async () => {
  let { store, basemaps, client } = setup();
  let first = Promise.withResolvers<string[]>();
  let second = Promise.withResolvers<string[]>();
  client.getEnrouteBasemapUpdates
    .mockReturnValueOnce(first.promise)
    .mockReturnValue(second.promise);
  store.current = catalog;
  basemaps.current = { generation: 1, sources: [] };
  flushSync();
  basemaps.current = { generation: 2, sources: [] };
  flushSync();
  first.resolve(['Europe/Germany.mbtiles']);
  await first.promise;
  expect(store.basemapUpdates).toBeNull();
  dispose?.();
  second.reject(new Error('late failure'));
  await second.promise.catch(() => {});
  expect(store.basemapUpdateError).toBe(false);
});

it.each(['catalog', 'subscription', 'inventory'] as const)(
  'reports a %s failure without treating missing state as no updates',
  (failure) => {
    let { store, basemaps, client } = setup();
    if (failure === 'catalog') store.current = { ...catalog, cached: null, error: true };
    if (failure === 'subscription') store.error = true;
    if (failure === 'inventory') {
      store.current = catalog;
      basemaps.error = true;
    }
    flushSync();
    expect(store.basemapUpdateError).toBe(true);
    expect(store.basemapUpdates).toBeNull();
    expect(client.getEnrouteBasemapUpdates).not.toHaveBeenCalled();
  },
);
