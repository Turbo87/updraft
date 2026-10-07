import type { Topic } from '#lib/protocol/generated/Topic.js';
import type { BondedBluetoothDevices } from './bonded-bluetooth-devices';
import type { EnrouteCatalogStatus, EnrouteDownloadStatus } from './index';

import { describe, expect, it, vi } from 'vitest';

import { instrumentsFixture } from '#lib/instruments.fixture.js';
import { FakeClient } from './fake';

it('delivers prepared arrival resources until the subscription closes', async () => {
  let client = new FakeClient();
  let listener = vi.fn();
  let subscription = client.subscribeArrivals([0, 0, 1, 1], listener);
  let update = { generation: 1, url: '/arrivals.geojson' };
  client.emitArrivals(update);
  expect(listener).toHaveBeenCalledExactlyOnceWith(update);
  await subscription.close();
  client.emitArrivals(update);
  expect(listener).toHaveBeenCalledTimes(1);
});

function collectTopics(client: FakeClient): Topic[] {
  let topics: Topic[] = [];
  client.subscribe((topic) => topics.push(topic));
  return topics;
}

function observeTopicChanges(client: FakeClient) {
  let onTopic = vi.fn<(topic: Topic) => void>();
  client.subscribe(onTopic);
  onTopic.mockClear();
  return onTopic;
}

function instruments(trackDegrees: number): Topic {
  return {
    topic: 'instruments',
    value: instrumentsFixture({
      gps: {
        position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
        altitudeMeters: null,
        groundSpeedMetersPerSecond: null,
        trackDegrees,
        fixTime: null,
        stale: false,
      },
    }),
  };
}

describe('FakeClient', () => {
  it('cancels native data selection in browser mode', async () => {
    let client = new FakeClient();

    await expect(client.selectDataFile()).resolves.toBeNull();
  });

  it('delivers emitted topics to a subscriber', () => {
    let client = new FakeClient();
    let received = collectTopics(client);
    received.length = 0;
    client.emit(instruments(270));

    expect(received).toEqual([instruments(270)]);
  });

  it('stops delivering after unsubscribe', () => {
    let client = new FakeClient();
    let onTopic = vi.fn();

    let unsubscribe = client.subscribe(onTopic);
    onTopic.mockClear();
    unsubscribe();
    client.emit(instruments(90));

    expect(onTopic).not.toHaveBeenCalled();
  });

  it('delivers onboarding topics when a subscriber connects', () => {
    let client = new FakeClient();
    let received = collectTopics(client);

    expect(received).toMatchSnapshot();
  });

  it('delivers the latest emitted snapshot topics to a new subscriber', () => {
    let client = new FakeClient();
    let saveFailed: Topic = { topic: 'taskSaveFailed', value: true };
    client.emit(instruments(90));
    client.emit(saveFailed);
    client.emit({
      topic: 'traffic',
      value: { type: 'delta', value: { upserts: [], removed: [] } },
    });

    let received = collectTopics(client);

    expect(received.filter((topic) => topic.topic === 'taskSaveFailed')).toEqual([saveFailed]);
    expect(received).toContainEqual(instruments(90));
    expect(received.filter((topic) => topic.topic === 'traffic')).toEqual([
      { topic: 'traffic', value: { type: 'snapshot', value: [] } },
    ]);
  });

  it('returns its configured bonded Bluetooth state', async () => {
    let bondedBluetoothDevices: BondedBluetoothDevices = {
      status: 'available',
      devices: [{ address: '00:11:22:33:44:55', name: 'Flight recorder' }],
    };
    let client = new FakeClient({ bondedBluetoothDevices });

    await expect(client.getBondedBluetoothDevices()).resolves.toEqual(bondedBluetoothDevices);
  });
});

it.each([
  ['subscribeBasemaps', 'emitBasemaps', 'local.mbtiles'],
  ['subscribeTerrain', 'emitTerrain', 'local.terrain'],
] as const)(
  'delivers current status and updates until each %s closes',
  async (subscribe, emit, sourceName) => {
    let client = new FakeClient();
    let first = vi.fn();
    let second = vi.fn();
    let subscription = client[subscribe](first);
    expect(first).toHaveBeenCalledExactlyOnceWith({ generation: 0, sources: [] });
    let status = {
      generation: 1,
      sources: [{ sourceName, type: 'disabled' as const }],
    };
    client[emit](status);
    expect(first).toHaveBeenLastCalledWith(status);
    let other = client[subscribe](second);
    expect(second).toHaveBeenCalledExactlyOnceWith(status);
    await subscription.close();
    await subscription.close();
    client[emit]({ generation: 2, sources: [] });
    expect(first).toHaveBeenCalledTimes(2);
    expect(second).toHaveBeenLastCalledWith({ generation: 2, sources: [] });
    await other.close();
  },
);

it('keeps terrain and basemap subscriptions independent', async () => {
  let client = new FakeClient();
  let basemaps = vi.fn();
  let terrain = vi.fn();
  let basemapSubscription = client.subscribeBasemaps(basemaps);
  let terrainSubscription = client.subscribeTerrain(terrain);
  client.emitTerrain({ generation: 2, sources: [] });
  expect(basemaps).toHaveBeenCalledExactlyOnceWith({ generation: 0, sources: [] });
  client.emitBasemaps({ generation: 1, sources: [] });
  expect(terrain.mock.calls).toEqual([
    [{ generation: 0, sources: [] }],
    [{ generation: 2, sources: [] }],
  ]);
  await basemapSubscription.close();
  await terrainSubscription.close();
});

it('delivers cached catalog and refresh states until each subscription closes', async () => {
  let client = new FakeClient();
  let first = vi.fn();
  let second = vi.fn();
  let subscription = client.subscribeEnrouteCatalog(first);
  expect(first).toHaveBeenCalledExactlyOnceWith({ cached: null, refreshing: false, error: false });
  let status: EnrouteCatalogStatus = {
    cached: {
      entries: [
        {
          path: 'Europe/Malta.mbtiles',
          countryCode: 'MT',
          continent: 'europe',
          size: 458752,
          publicationDate: '2026-09-08',
        },
      ],
      checkedAt: 1788825600000,
    },
    refreshing: true,
    error: false,
  };
  client.emitEnrouteCatalog(status);
  expect(first).toHaveBeenLastCalledWith(status);
  let other = client.subscribeEnrouteCatalog(second);
  expect(second).toHaveBeenCalledExactlyOnceWith(status);
  await subscription.close();
  await subscription.close();
  let failed = { ...status, refreshing: false, error: true };
  client.emitEnrouteCatalog(failed);
  expect(first).toHaveBeenCalledTimes(2);
  expect(second).toHaveBeenLastCalledWith(failed);
  await other.close();
});

it('delivers download snapshots until each subscription closes', async () => {
  let client = new FakeClient();
  let first = vi.fn();
  let second = vi.fn();
  let subscription = client.subscribeEnrouteDownloads(first);
  expect(first).toHaveBeenCalledExactlyOnceWith([]);
  let status: EnrouteDownloadStatus[] = [
    { path: 'Europe/Malta.mbtiles', type: 'downloading', downloaded: 12, total: 100 },
    { path: 'Europe/Germany.mbtiles', type: 'queued' },
    { path: 'Europe/France.mbtiles', type: 'failed' },
  ];
  client.emitEnrouteDownloads(status);
  expect(first).toHaveBeenLastCalledWith(status);
  let other = client.subscribeEnrouteDownloads(second);
  expect(second).toHaveBeenCalledExactlyOnceWith(status);
  await subscription.close();
  await subscription.close();
  client.emitEnrouteDownloads([]);
  expect(first).toHaveBeenCalledTimes(2);
  expect(second).toHaveBeenLastCalledWith([]);
  await other.close();
  client.emitEnrouteDownloads(status);
  expect(second).toHaveBeenCalledTimes(2);
});

it('records navigation commands and replies with queued results without publishing', async () => {
  let client = new FakeClient();
  let onTopic = observeTopicChanges(client);
  let target = { type: 'traffic', id: 'icao:ABC123' } as const;
  client.queueNavigationReplies(false);

  let replies = [
    await client.pinTarget(target),
    await client.unpinTarget(0),
    await client.setNavigationTarget(target),
    await client.changeTask({ type: 'stop' }),
    await client.saveTask(),
  ];

  expect(replies).toEqual([false, true, true, true, true]);
  expect(client.navigationCommands).toEqual([
    ['pinTarget', target],
    ['unpinTarget', 0],
    ['setNavigationTarget', target],
    ['changeTask', { type: 'stop' }],
    ['saveTask'],
  ]);
  expect(onTopic).not.toHaveBeenCalled();
});

it('records settings commands without validating or publishing', async () => {
  let client = new FakeClient();
  let onTopic = observeTopicChanges(client);

  await client.changeSetting({ type: 'arrivalReserve', reserve: -1 });
  await client.setPolar('Unknown glider');
  await client.setEnergyCompensation(false);
  await client.setFlarmPositionCorrection(false);
  await client.setMacCready(NaN);
  await client.setBugs(100);
  await client.setBallast(-1);

  expect(client.settingsCommands).toEqual([
    ['changeSetting', { type: 'arrivalReserve', reserve: -1 }],
    ['setPolar', 'Unknown glider'],
    ['setEnergyCompensation', false],
    ['setFlarmPositionCorrection', false],
    ['setMacCready', NaN],
    ['setBugs', 100],
    ['setBallast', -1],
  ]);
  expect(onTopic).not.toHaveBeenCalled();
});

it('records external device and data file commands without publishing', async () => {
  let client = new FakeClient();
  let onTopic = observeTopicChanges(client);
  let spec = { type: 'tcp', host: '127.0.0.1', port: 4353 } as const;

  let deviceId = await client.addExternalDevice(spec);
  await client.editExternalDevice(deviceId, { ...spec, port: 10110 });
  await client.setExternalDeviceEnabled(deviceId, false);
  await client.deleteExternalDevice(deviceId);
  await client.setAirspaceEnabled('a.txt', true);
  await client.removeAirspace('a.txt');
  await client.setWaypointsEnabled('b.cup', true);
  await client.removeWaypoints('b.cup');

  expect(client.externalDeviceCommands).toEqual([
    ['addExternalDevice', spec],
    ['editExternalDevice', deviceId, { ...spec, port: 10110 }],
    ['setExternalDeviceEnabled', deviceId, false],
    ['deleteExternalDevice', deviceId],
  ]);
  expect(client.dataFileCommands).toEqual([
    ['setAirspaceEnabled', 'a.txt', true],
    ['removeAirspace', 'a.txt'],
    ['setWaypointsEnabled', 'b.cup', true],
    ['removeWaypoints', 'b.cup'],
  ]);
  expect(onTopic).not.toHaveBeenCalled();
});
