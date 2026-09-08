import { mockConvertFileSrc } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { TauriClient } from './tauri';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  channels: [] as { onmessage: (value: unknown) => void }[],
}));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tauri-apps/api/core')>()),
  invoke: mocks.invoke,
  Channel: class {
    id = mocks.channels.length;
    onmessage: (value: unknown) => void = () => {};
    constructor() {
      mocks.channels.push(this);
    }
  },
}));

beforeEach(() => {
  vi.stubGlobal('window', {});
  mockConvertFileSrc('macos');
  mocks.invoke.mockReset();
  mocks.channels.length = 0;
});

afterEach(() => vi.unstubAllGlobals());

it.each([
  ['setAirspaceEnabled', 'set_airspace_enabled'],
  ['setWaypointsEnabled', 'set_waypoints_enabled'],
  ['setBasemapEnabled', 'set_basemap_enabled'],
  ['setTerrainEnabled', 'set_terrain_enabled'],
] as const)('forwards %s and propagates failures', async (method, command) => {
  let client = new TauriClient();
  mocks.invoke.mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error('storage failed'));
  await client[method]('local.txt', false);
  await expect(client[method]('local.txt', true)).rejects.toThrow('storage failed');
  expect(mocks.invoke.mock.calls).toEqual([
    [command, { sourceName: 'local.txt', enabled: false }],
    [command, { sourceName: 'local.txt', enabled: true }],
  ]);
});

it.each(['macos', 'windows'] as const)('builds arrival URLs on %s', async (os) => {
  mockConvertFileSrc(os);
  let origin = os === 'windows' ? 'http://updraft.localhost' : 'updraft://localhost';
  let start = Promise.withResolvers<string>();
  mocks.invoke.mockReturnValueOnce(start.promise).mockResolvedValue(undefined);
  let update = vi.fn();
  let error = vi.fn();
  let bounds: [number, number, number, number] = [0, 0, 1, 1];
  let subscription = new TauriClient().subscribeArrivals(bounds, update, error);
  expect(mocks.invoke).toHaveBeenCalledWith('start_arrivals', {
    bounds,
    channel: mocks.channels[0],
  });
  mocks.channels[0].onmessage({ type: 'ready', generation: 7, revision: 1 });
  mocks.channels[0].onmessage({ type: 'ready', generation: 7, revision: 2 });
  expect(update).not.toHaveBeenCalled();
  start.resolve('12');
  await subscription.updateViewport([1, 1, 2, 2]);
  expect(update).toHaveBeenCalledExactlyOnceWith({
    generation: 7,
    url: `${origin}/arrivals/12.geojson?v=2`,
  });
  expect(mocks.invoke).toHaveBeenLastCalledWith('update_arrival_viewport', {
    id: '12',
    bounds: [1, 1, 2, 2],
  });
  mocks.channels[0].onmessage({ type: 'ready', generation: 8, revision: 3 });
  expect(update).toHaveBeenLastCalledWith({
    generation: 8,
    url: `${origin}/arrivals/12.geojson?v=3`,
  });
  mocks.channels[0].onmessage({ type: 'failed' });
  expect(error).toHaveBeenCalledExactlyOnceWith(new Error('Arrival worker stopped'));
  await subscription.close();
  await subscription.close();
  expect(mocks.invoke.mock.calls.filter(([command]) => command === 'stop_arrivals')).toEqual([
    ['stop_arrivals', { id: '12' }],
  ]);
});

it('discards a buffered notification when the worker fails before startup completes', async () => {
  let start = Promise.withResolvers<string>();
  mocks.invoke.mockReturnValueOnce(start.promise).mockResolvedValue(undefined);
  let update = vi.fn();
  let error = vi.fn();
  let subscription = new TauriClient().subscribeArrivals([0, 0, 1, 1], update, error);
  mocks.channels[0].onmessage({ type: 'ready', generation: 1, revision: 1 });
  mocks.channels[0].onmessage({ type: 'failed' });
  start.resolve('3');
  await subscription.updateViewport([0, 0, 1, 1]);
  expect(update).not.toHaveBeenCalled();
  expect(error).toHaveBeenCalledExactlyOnceWith(new Error('Arrival worker stopped'));
  await subscription.close();
});

it('closes a subscription that finishes starting after unmount', async () => {
  let start = Promise.withResolvers<string>();
  mocks.invoke.mockReturnValueOnce(start.promise).mockResolvedValue(undefined);
  let update = vi.fn();
  let error = vi.fn();
  let subscription = new TauriClient().subscribeArrivals([0, 0, 1, 1], update, error);
  let movement = subscription.updateViewport([1, 1, 2, 2]);
  let closing = subscription.close();
  mocks.channels[0].onmessage({ type: 'ready', generation: 1, revision: 1 });
  start.resolve('3');
  await Promise.all([closing, movement]);
  expect(mocks.invoke.mock.calls.map(([command]) => command)).toEqual([
    'start_arrivals',
    'stop_arrivals',
  ]);
  expect(update).not.toHaveBeenCalled();
  expect(error).not.toHaveBeenCalled();
});

it('reports startup failures and propagates command failures', async () => {
  mocks.invoke.mockRejectedValueOnce(new Error('start failed'));
  let error = vi.fn();
  let failed = new TauriClient().subscribeArrivals([0, 0, 1, 1], vi.fn(), error);
  await failed.updateViewport([0, 0, 1, 1]);
  expect(error).toHaveBeenCalledExactlyOnceWith(new Error('start failed'));
  await failed.close();
  expect(mocks.invoke).toHaveBeenCalledTimes(1);
  mocks.invoke.mockResolvedValueOnce('4').mockRejectedValue(new Error('command failed'));
  let subscription = new TauriClient().subscribeArrivals([0, 0, 1, 1], vi.fn(), error);
  await expect(subscription.updateViewport([0, 0, 2, 2])).rejects.toThrow('command failed');
  await expect(subscription.close()).rejects.toThrow('command failed');
});

it('forwards data selection, import, and discard commands', async () => {
  let client = new TauriClient();
  let selected = { selectionId: '4', sourceName: 'local.cup', dataType: 'waypoints' };
  mocks.invoke
    .mockResolvedValueOnce(selected)
    .mockResolvedValueOnce(selected)
    .mockResolvedValueOnce(undefined);
  expect(await client.selectDataFile()).toEqual(selected);
  expect(await client.importDataFile('4')).toEqual(selected);
  await client.discardDataFile('4');
  expect(mocks.invoke.mock.calls).toEqual([
    ['select_data_file'],
    ['import_data_file', { selectionId: '4' }],
    ['discard_data_file', { selectionId: '4' }],
  ]);
  mocks.invoke.mockRejectedValue(new Error('read failed'));
  await expect(client.selectDataFile()).rejects.toThrow('read failed');
  await expect(client.importDataFile('4')).rejects.toThrow('read failed');
  await expect(client.discardDataFile('4')).rejects.toThrow('read failed');
});

describe.each([
  [
    'subscribeBasemaps',
    'subscribe_basemaps',
    'unsubscribe_basemaps',
    { generation: 0, sources: [{ sourceName: 'local.mbtiles', type: 'active' }] },
  ],
  [
    'subscribeTerrain',
    'subscribe_terrain',
    'unsubscribe_terrain',
    { generation: 0, sources: [{ sourceName: 'local.terrain', type: 'active' }] },
  ],
  [
    'subscribeEnrouteCatalog',
    'subscribe_enroute_catalog',
    'unsubscribe_enroute_catalog',
    { cached: null, refreshing: true, error: false },
  ],
] as const)('%s', (method, subscribe, unsubscribe, status) => {
  it('delivers status and closes native registration after pending startup', async () => {
    let start = Promise.withResolvers<void>();
    mocks.invoke.mockReturnValueOnce(start.promise).mockResolvedValue(undefined);
    let update = vi.fn();
    let error = vi.fn();
    let subscription = new TauriClient()[method](update, error);
    let channel = mocks.channels[0];
    channel.onmessage(status);
    expect(update).toHaveBeenCalledExactlyOnceWith(status);
    let closing = subscription.close();
    expect(subscription.close()).toBe(closing);
    channel.onmessage(status);
    expect(update).toHaveBeenCalledTimes(1);
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    start.resolve();
    await closing;
    expect(mocks.invoke.mock.calls).toEqual([
      [subscribe, { channel }],
      [unsubscribe, { channelId: 0 }],
    ]);
    expect(error).not.toHaveBeenCalled();
  });

  it.each([false, true])('handles startup failure with closed=%s', async (closed) => {
    let start = Promise.withResolvers<void>();
    mocks.invoke.mockReturnValueOnce(start.promise);
    let error = vi.fn();
    let update = vi.fn();
    let subscription = new TauriClient()[method](update, error);
    let closing = closed ? subscription.close() : undefined;
    let failure = new Error('subscription failed');
    start.reject(failure);
    await start.promise.catch(() => {});
    mocks.channels[0].onmessage(status);
    expect(update).not.toHaveBeenCalled();
    await (closing ?? subscription.close());
    expect(error.mock.calls).toEqual(closed ? [] : [[failure]]);
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
  });

  it('propagates native unsubscribe failures', async () => {
    mocks.invoke
      .mockResolvedValueOnce(undefined)
      .mockRejectedValueOnce(new Error('unsubscribe failed'));
    let subscription = new TauriClient()[method](vi.fn(), vi.fn());
    await expect(subscription.close()).rejects.toThrow('unsubscribe failed');
  });
});

it('forwards terrain removal and propagates failures', async () => {
  let client = new TauriClient();
  mocks.invoke.mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error('storage failed'));
  await client.removeTerrain('local.terrain');
  await expect(client.removeTerrain('local.terrain')).rejects.toThrow('storage failed');
  expect(mocks.invoke.mock.calls).toEqual([
    ['remove_terrain', { sourceName: 'local.terrain' }],
    ['remove_terrain', { sourceName: 'local.terrain' }],
  ]);
});

it('forwards catalog refresh and propagates failures', async () => {
  let client = new TauriClient();
  mocks.invoke.mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error('refresh failed'));
  await client.refreshEnrouteCatalog();
  await expect(client.refreshEnrouteCatalog()).rejects.toThrow('refresh failed');
  expect(mocks.invoke.mock.calls).toEqual([
    ['refresh_enroute_catalog'],
    ['refresh_enroute_catalog'],
  ]);
});
