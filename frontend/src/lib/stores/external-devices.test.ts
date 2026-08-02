import type { Topic } from '$lib/protocol/generated/Topic';

import { describe, expect, it } from 'vitest';

import { ExternalDevicesStore } from './external-devices.svelte';

describe('ExternalDevicesStore', () => {
  it('marks the first external-device topic as initialized', () => {
    let store = new ExternalDevicesStore();

    expect(store.initialized).toBe(false);

    store.apply({
      topic: 'externalDevices',
      value: [{ deviceId: 1, enabled: true, type: 'tcp', host: '127.0.0.1', port: 4353 }],
    });

    expect(store.initialized).toBe(true);
    expect(store.current).toEqual([
      { deviceId: 1, enabled: true, type: 'tcp', host: '127.0.0.1', port: 4353 },
    ]);
  });

  it('replaces the complete ordered device list', () => {
    let store = new ExternalDevicesStore();
    store.apply({
      topic: 'externalDevices',
      value: [{ deviceId: 1, enabled: true, type: 'tcp', host: '127.0.0.1', port: 4353 }],
    });

    store.apply({
      topic: 'externalDevices',
      value: [
        {
          deviceId: 2,
          enabled: false,
          type: 'bluetooth',
          address: '00:11:22:33:44:55',
        },
        { deviceId: 1, enabled: true, type: 'tcp', host: '192.0.2.1', port: 10110 },
      ],
    });

    expect(store.current).toEqual([
      {
        deviceId: 2,
        enabled: false,
        type: 'bluetooth',
        address: '00:11:22:33:44:55',
      },
      { deviceId: 1, enabled: true, type: 'tcp', host: '192.0.2.1', port: 10110 },
    ]);
  });

  it('ignores unrelated topics before initialization', () => {
    let store = new ExternalDevicesStore();
    let topic: Topic = {
      topic: 'settings',
      value: {
        locale: null,
        units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      },
    };

    store.apply(topic);

    expect(store.initialized).toBe(false);
    expect(store.current).toEqual([]);
  });
});
