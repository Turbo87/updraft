import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { BondedBluetoothDevices } from './bonded-bluetooth-devices';

import { describe, expect, it, vi } from 'vitest';

import { FakeClient } from './fake';

type ExternalDevicesTopic = Extract<Topic, { topic: 'externalDevices' }>;

function externalDeviceTopics(topics: Topic[]): ExternalDevicesTopic[] {
  return topics.filter((topic): topic is ExternalDevicesTopic => topic.topic === 'externalDevices');
}

function configuredDevices(): PublishedExternalDevice[] {
  return [
    { deviceId: 4, enabled: true, type: 'tcp', host: '127.0.0.1', port: 4353 },
    {
      deviceId: 7,
      enabled: false,
      type: 'bluetooth',
      address: '00:11:22:33:44:55',
    },
  ];
}

function instruments(trackDegrees: number): Topic {
  return {
    topic: 'instruments',
    value: {
      gps: {
        position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
        altitudeMeters: null,
        groundSpeedMetersPerSecond: null,
        trackDegrees,
        fixTime: null,
        stale: false,
      },
      pressureAltitude: null,
      trueAirspeed: null,
    },
  };
}

describe('FakeClient', () => {
  it('cancels native airspace import in browser mode', async () => {
    let client = new FakeClient();

    await expect(client.importAirspace()).resolves.toEqual({ type: 'cancelled' });
  });

  it('delivers emitted topics to a subscriber', () => {
    let client = new FakeClient();
    let received: Topic[] = [];

    client.subscribe((topic) => received.push(topic));
    received = [];
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
    let received: Topic[] = [];

    client.subscribe((topic) => received.push(topic));

    expect(received).toMatchSnapshot();
  });

  it('publishes an explicit locale through the settings topic', async () => {
    let client = new FakeClient();
    let received: Topic[] = [];
    client.subscribe((topic) => received.push(topic));

    await client.setLocale('de');

    expect(received.at(-1)).toEqual({
      topic: 'settings',
      value: {
        locale: 'de',
        units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      },
    });
  });

  it('does not republish the active explicit locale', async () => {
    let client = new FakeClient();
    let received: Topic[] = [];
    client.subscribe((topic) => received.push(topic));
    received = [];

    await client.setLocale('de');
    await client.setLocale('de');

    expect(received).toHaveLength(1);
  });

  it('publishes complete unit selections through the settings topic', async () => {
    let client = new FakeClient();
    let received: Topic[] = [];
    client.subscribe((topic) => received.push(topic));
    await client.setLocale('de');
    received = [];

    await client.setUnits({
      altitude: 'ft',
      distance: 'nm',
      speed: 'kt',
      verticalSpeed: 'ft/min',
    });

    expect(received.at(-1)).toEqual({
      topic: 'settings',
      value: {
        locale: 'de',
        units: { altitude: 'ft', distance: 'nm', speed: 'kt', verticalSpeed: 'ft/min' },
      },
    });
  });

  it('does not republish equal unit selections', async () => {
    let client = new FakeClient();
    let received: Topic[] = [];
    client.subscribe((topic) => received.push(topic));
    received = [];

    await client.setUnits({
      altitude: 'm',
      distance: 'km',
      speed: 'km/h',
      verticalSpeed: 'm/s',
    });

    expect(received).toEqual([]);
  });

  it('allocates device IDs and publishes complete authoritative topics', async () => {
    let client = new FakeClient();
    let received: Topic[] = [];
    client.subscribe((topic) => received.push(topic));
    received = [];

    let tcpId = await client.addExternalDevice({ type: 'tcp', host: '127.0.0.1', port: 4353 });
    let bluetoothId = await client.addExternalDevice({
      type: 'bluetooth',
      address: '00:11:22:33:44:55',
    });

    expect([tcpId, bluetoothId]).toEqual([1, 2]);
    expect(externalDeviceTopics(received)).toMatchSnapshot();
  });

  it('allocates a fresh ID after configured devices', async () => {
    let client = new FakeClient({ externalDevices: configuredDevices() });

    await expect(
      client.addExternalDevice({ type: 'tcp', host: '192.0.2.1', port: 10110 }),
    ).resolves.toBe(8);
  });

  it('edits a device without changing its ID, enabled state, or position', async () => {
    let client = new FakeClient({ externalDevices: configuredDevices() });
    let received: Topic[] = [];
    client.subscribe((topic) => received.push(topic));
    received = [];

    await client.editExternalDevice(4, {
      type: 'bluetooth',
      address: 'AA:BB:CC:DD:EE:FF',
    });

    expect(externalDeviceTopics(received)).toMatchSnapshot();
  });

  it('publishes enable, disable, and delete changes', async () => {
    let client = new FakeClient({ externalDevices: configuredDevices() });
    let received: Topic[] = [];
    client.subscribe((topic) => received.push(topic));
    received = [];

    await client.setExternalDeviceEnabled(4, false);
    await client.setExternalDeviceEnabled(7, true);
    await client.deleteExternalDevice(4);

    expect(externalDeviceTopics(received).map((topic) => topic.value)).toMatchSnapshot();
  });

  it('does not publish identical edits or enabled states', async () => {
    let client = new FakeClient({ externalDevices: configuredDevices() });
    let received: Topic[] = [];
    client.subscribe((topic) => received.push(topic));
    received = [];

    await client.editExternalDevice(4, { type: 'tcp', host: '127.0.0.1', port: 4353 });
    await client.setExternalDeviceEnabled(4, true);

    expect(externalDeviceTopics(received)).toEqual([]);
  });

  it('rejects unknown device IDs without changing authoritative state', async () => {
    let devices = configuredDevices();
    let client = new FakeClient({ externalDevices: devices });
    let received: Topic[] = [];
    client.subscribe((topic) => received.push(topic));
    received = [];

    await expect(
      client.editExternalDevice(99, { type: 'tcp', host: '192.0.2.1', port: 10110 }),
    ).rejects.toEqual({ kind: 'unknownExternalDevice', deviceId: 99 });
    await expect(client.setExternalDeviceEnabled(99, false)).rejects.toEqual({
      kind: 'unknownExternalDevice',
      deviceId: 99,
    });
    await expect(client.deleteExternalDevice(99)).rejects.toEqual({
      kind: 'unknownExternalDevice',
      deviceId: 99,
    });

    expect(externalDeviceTopics(received)).toEqual([]);

    let current: PublishedExternalDevice[] = [];
    client.subscribe((topic) => {
      if (topic.topic === 'externalDevices') current = topic.value;
    });
    expect(current).toEqual(devices);
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
