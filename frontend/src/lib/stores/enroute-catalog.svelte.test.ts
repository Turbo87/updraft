import type { EnrouteCatalogStatus } from '$lib/client';

import { flushSync } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';

import { BasemapsStore } from './basemaps.svelte';
import { EnrouteCatalogStore } from './enroute-catalog.svelte';
import { TerrainStore } from './terrain.svelte';

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
  let terrain = new TerrainStore();
  terrain.current = { generation: 0, sources: [] };
  let client = {
    getEnrouteBasemapUpdates: vi.fn<() => Promise<string[]>>(),
    getEnrouteTerrainUpdates: vi.fn(async () => [] as string[]),
  };
  dispose = store.watchUpdates(client, basemaps, terrain);
  return { store, basemaps, terrain, client };
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
  await vi.waitFor(() => expect(store.updates).toEqual(['Europe/Germany.mbtiles']));
  expect(store.updateError).toBe(false);
  client.getEnrouteBasemapUpdates.mockResolvedValue([]);
  basemaps.current = { generation: 2, sources: [] };
  flushSync();
  await vi.waitFor(() => expect(store.updates).toEqual([]));
  expect(client.getEnrouteBasemapUpdates).toHaveBeenCalledTimes(2);
});

it('distinguishes failed checks from no updates and recovers after retry', async () => {
  let { store, basemaps, client } = setup();
  basemaps.current = { generation: 1, sources: [] };
  store.current = catalog;
  client.getEnrouteBasemapUpdates.mockRejectedValue(new Error('metadata failed'));
  flushSync();
  await vi.waitFor(() => expect(store.updateError).toBe(true));
  expect(store.updates).toBeNull();
  client.getEnrouteBasemapUpdates.mockResolvedValue([]);
  store.current = { ...catalog, refreshing: true };
  flushSync();
  expect(client.getEnrouteBasemapUpdates).toHaveBeenCalledTimes(1);
  store.current = { ...catalog };
  flushSync();
  await vi.waitFor(() => expect(store.updates).toEqual([]));
  expect(store.updateError).toBe(false);
  store.current = { ...catalog, error: true };
  flushSync();
  await vi.waitFor(() => expect(store.updates).toEqual([]));
  expect(store.updateError).toBe(true);
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
  expect(store.updates).toBeNull();
  dispose?.();
  second.reject(new Error('late failure'));
  await second.promise.catch(() => {});
  expect(store.updateError).toBe(false);
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
    expect(store.updateError).toBe(true);
    expect(store.updates).toBeNull();
    expect(client.getEnrouteBasemapUpdates).not.toHaveBeenCalled();
  },
);

it('combines both inventories and rechecks terrain changes', async () => {
  let { store, basemaps, terrain, client } = setup();
  client.getEnrouteBasemapUpdates.mockResolvedValue(['Europe/France.mbtiles']);
  client.getEnrouteTerrainUpdates.mockResolvedValue(['Europe/France.terrain']);
  store.current = catalog;
  basemaps.current = { generation: 1, sources: [] };
  terrain.current = {
    generation: 1,
    sources: [{ sourceName: 'enroute/Europe/France.terrain', type: 'disabled' }],
  };
  flushSync();
  await vi.waitFor(() =>
    expect(store.updates).toEqual(['Europe/France.mbtiles', 'Europe/France.terrain']),
  );
  client.getEnrouteTerrainUpdates.mockRejectedValueOnce(new Error('metadata failed'));
  terrain.current = { ...terrain.current, generation: 2 };
  flushSync();
  await vi.waitFor(() => expect(store.updateError).toBe(true));
  expect(store.updates).toBeNull();
  terrain.current = { generation: 3, sources: [] };
  client.getEnrouteTerrainUpdates.mockResolvedValue([]);
  flushSync();
  await vi.waitFor(() => expect(store.updates).toEqual(['Europe/France.mbtiles']));
  expect(store.updateError).toBe(false);
});

it('waits for terrain inventory and reports its subscription failure', async () => {
  let { store, basemaps, terrain, client } = setup();
  store.current = catalog;
  basemaps.current = { generation: 0, sources: [] };
  terrain.current = null;
  flushSync();
  expect(client.getEnrouteTerrainUpdates).not.toHaveBeenCalled();
  expect(store.updates).toBeNull();
  terrain.error = true;
  flushSync();
  expect(store.updateError).toBe(true);
});
