import { expect, it, vi } from 'vitest';

import { AirspaceStore } from './airspace.svelte';
import { BasemapsStore } from './basemaps.svelte';
import { DataActivation } from './data-activation.svelte';
import { WaypointsStore } from './waypoints.svelte';

it('discards activation errors when a file is removed', async () => {
  let airspace = new AirspaceStore();
  let waypoints = new WaypointsStore();
  airspace.current = { generation: 1, sources: [{ type: 'disabled', sourceName: 'local' }] };
  let client = {
    setAirspaceEnabled: vi.fn().mockRejectedValue(new Error('storage failed')),
    setBasemapEnabled: vi.fn(),
    setWaypointsEnabled: vi.fn(),
  };
  let activation = new DataActivation(client, airspace, waypoints, new BasemapsStore());
  activation.setEnabled('airspace', 'local', true);
  await vi.waitFor(() => expect(activation.pending).toBe(false));
  expect(activation.hasError('airspace', 'local')).toBe(true);
  let removed = { topic: 'airspace' as const, value: { generation: 2, sources: [] } };
  airspace.apply(removed);
  activation.apply(removed);
  expect(activation.hasError('airspace', 'local')).toBe(false);
  let command = Promise.withResolvers<void>();
  client.setAirspaceEnabled.mockReturnValue(command.promise);
  airspace.current = { generation: 3, sources: [{ type: 'disabled', sourceName: 'local' }] };
  activation.setEnabled('airspace', 'local', true);
  let removedAgain = { topic: 'airspace' as const, value: { generation: 4, sources: [] } };
  airspace.apply(removedAgain);
  activation.apply(removedAgain);
  command.reject(new Error('file removed'));
  await vi.waitFor(() => expect(activation.pending).toBe(false));
  expect(activation.hasError('airspace', 'local')).toBe(false);
});

it.each(['airspace', 'basemap'] as const)(
  'keeps the latest %s choice while replies arrive separately',
  async (type) => {
    let airspace = new AirspaceStore();
    let waypoints = new WaypointsStore();
    let basemaps = new BasemapsStore();
    let store = type === 'airspace' ? airspace : basemaps;
    store.current = {
      generation: 1,
      sources: [{ type: 'active', sourceName: 'local', airspaceCount: 1 }],
    };
    let first = Promise.withResolvers<void>();
    let second = Promise.withResolvers<void>();
    let setEnabled = vi.fn().mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    let client = {
      setAirspaceEnabled: setEnabled,
      setBasemapEnabled: setEnabled,
      setWaypointsEnabled: vi.fn(),
    };
    let activation = new DataActivation(client, airspace, waypoints, basemaps);
    activation.setEnabled(type, 'local', false);
    activation.setEnabled(type, 'local', true);
    expect(activation.isEnabled(type, store.current!.sources[0])).toBe(true);
    expect(setEnabled.mock.calls).toEqual([['local', false]]);
    first.resolve();
    await Promise.resolve();
    expect(setEnabled).toHaveBeenCalledTimes(1);
    let disabled = {
      topic: type,
      value: { generation: 2, sources: [{ type: 'disabled' as const, sourceName: 'local' }] },
    };
    store.current = disabled.value;
    activation.apply(disabled);
    await vi.waitFor(() => expect(setEnabled).toHaveBeenCalledTimes(2));
    expect(activation.isEnabled(type, store.current!.sources[0])).toBe(true);
    second.reject(new Error('storage failed'));
    await vi.waitFor(() => expect(activation.pending).toBe(false));
    expect(activation.isEnabled(type, store.current!.sources[0])).toBe(false);
    expect(activation.hasError(type, 'local')).toBe(true);
    setEnabled.mockResolvedValue(undefined);
    activation.setEnabled(type, 'local', true);
    activation.apply(disabled);
    expect(activation.isEnabled(type, store.current!.sources[0])).toBe(true);
    let enabled = {
      topic: type,
      value: {
        generation: 3,
        sources: [
          { type: 'unavailable' as const, sourceName: 'local', error: 'parseFailed' as const },
        ],
      },
    };
    store.current = enabled.value;
    activation.apply(enabled);
    await vi.waitFor(() => expect(activation.pending).toBe(false));
    expect(activation.hasError(type, 'local')).toBe(false);
  },
);

it('remembers a matching publication before the command reply', async () => {
  let airspace = new AirspaceStore();
  let waypoints = new WaypointsStore();
  airspace.current = { generation: 1, sources: [{ type: 'disabled', sourceName: 'local' }] };
  let command = Promise.withResolvers<void>();
  let client = {
    setAirspaceEnabled: vi.fn().mockReturnValue(command.promise),
    setBasemapEnabled: vi.fn(),
    setWaypointsEnabled: vi.fn(),
  };
  let activation = new DataActivation(client, airspace, waypoints, new BasemapsStore());
  activation.setEnabled('airspace', 'local', true);
  let enabled = {
    topic: 'airspace' as const,
    value: {
      generation: 2,
      sources: [{ type: 'active' as const, sourceName: 'local', airspaceCount: 1 }],
    },
  };
  airspace.apply(enabled);
  activation.apply(enabled);
  let disabled = {
    topic: 'airspace' as const,
    value: { generation: 3, sources: [{ type: 'disabled' as const, sourceName: 'local' }] },
  };
  airspace.apply(disabled);
  activation.apply(disabled);
  command.resolve();
  await vi.waitFor(() => expect(activation.pending).toBe(false));
  expect(activation.isEnabled('airspace', airspace.current.sources[0])).toBe(false);
});

it('serializes files, accepts parse failures as enabled, and ignores superseded failures', async () => {
  let airspace = new AirspaceStore();
  let waypoints = new WaypointsStore();
  airspace.current = { generation: 1, sources: [{ type: 'disabled', sourceName: 'local' }] };
  waypoints.current = { generation: 1, sources: [{ type: 'disabled', sourceName: 'local' }] };
  let first = Promise.withResolvers<void>();
  let client = {
    setAirspaceEnabled: vi.fn().mockReturnValueOnce(first.promise).mockResolvedValue(undefined),
    setBasemapEnabled: vi.fn(),
    setWaypointsEnabled: vi.fn().mockResolvedValue(undefined),
  };
  let activation = new DataActivation(client, airspace, waypoints, new BasemapsStore());
  activation.setEnabled('airspace', 'local', true);
  activation.setEnabled('waypoints', 'local', true);
  activation.setEnabled('airspace', 'local', false);
  expect(client.setWaypointsEnabled).not.toHaveBeenCalled();
  first.reject(new Error('busy'));
  await vi.waitFor(() => expect(client.setWaypointsEnabled).toHaveBeenCalledOnce());
  expect(activation.hasError('airspace', 'local')).toBe(false);
  let parsed = {
    topic: 'waypoints' as const,
    value: {
      generation: 2,
      sources: [
        { type: 'unavailable' as const, sourceName: 'local', error: 'parseFailed' as const },
      ],
    },
  };
  waypoints.apply(parsed);
  activation.apply(parsed);
  await vi.waitFor(() => expect(client.setAirspaceEnabled).toHaveBeenCalledTimes(2));
  let disabled = {
    topic: 'airspace' as const,
    value: { generation: 2, sources: [{ type: 'disabled' as const, sourceName: 'local' }] },
  };
  airspace.apply(disabled);
  activation.apply(disabled);
  await vi.waitFor(() => expect(activation.pending).toBe(false));
  expect(activation.hasError('waypoints', 'local')).toBe(false);
  expect(activation.isEnabled('waypoints', waypoints.current.sources[0])).toBe(true);
});
