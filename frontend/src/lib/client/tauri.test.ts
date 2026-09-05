import { mockConvertFileSrc } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';

import { TauriClient } from './tauri';

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  channels: [] as { onmessage: (value: unknown) => void }[],
}));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tauri-apps/api/core')>()),
  invoke: mocks.invoke,
  Channel: class {
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
