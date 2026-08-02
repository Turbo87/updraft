import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';
import type { ExternalDeviceId } from '$lib/protocol/generated/ExternalDeviceId';
import type { Locale } from '$lib/protocol/generated/Locale';
import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';
import type { Settings } from '$lib/protocol/generated/Settings';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
import type { BondedBluetoothDevices } from './bonded-bluetooth-devices';
import type { ImportAirspaceResult, TopicListener, UpdraftClient } from './index';

/** Initial platform and external-device state for browser development. */
export type FakeClientOptions = {
  externalDevices?: readonly PublishedExternalDevice[];
  bondedBluetoothDevices?: BondedBluetoothDevices;
};

function hasConnectionSpec(device: PublishedExternalDevice, spec: ConnectionSpec): boolean {
  if (device.type === 'tcp' && spec.type === 'tcp') {
    return device.host === spec.host && device.port === spec.port;
  }
  if (device.type === 'bluetooth' && spec.type === 'bluetooth') {
    return device.address === spec.address && device.serviceUuid === spec.serviceUuid;
  }
  return false;
}

function unknownExternalDeviceError(deviceId: ExternalDeviceId): {
  kind: 'unknownExternalDevice';
  deviceId: ExternalDeviceId;
} {
  return { kind: 'unknownExternalDevice', deviceId };
}

/** Drives the frontend without a Rust process behind it. */
export class FakeClient implements UpdraftClient {
  #listeners = new Set<TopicListener>();
  #externalDevices: PublishedExternalDevice[];
  #nextExternalDeviceId: ExternalDeviceId;
  #bondedBluetoothDevices: BondedBluetoothDevices;
  #settings: Settings = {
    locale: null,
    units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
  };

  constructor(options: FakeClientOptions = {}) {
    this.#externalDevices = options.externalDevices?.map((device) => ({ ...device })) ?? [];
    this.#nextExternalDeviceId = this.#externalDevices.reduce(
      (nextId, device) => Math.max(nextId, device.deviceId + 1),
      1,
    );
    this.#bondedBluetoothDevices = options.bondedBluetoothDevices ?? { status: 'unsupported' };
  }

  async importAirspace(): Promise<ImportAirspaceResult> {
    return { type: 'cancelled' };
  }

  async removeAirspace(): Promise<void> {}

  subscribe(onTopic: TopicListener): () => void {
    this.#listeners.add(onTopic);
    onTopic({ topic: 'settings', value: this.#settings });
    onTopic({ topic: 'externalDevices', value: this.#externalDevices });
    onTopic({ topic: 'traffic', value: { type: 'snapshot', value: [] } });
    onTopic({ topic: 'airspace', value: { type: 'none' } });

    return () => {
      this.#listeners.delete(onTopic);
    };
  }

  async addExternalDevice(spec: ConnectionSpec): Promise<ExternalDeviceId> {
    let deviceId = this.#nextExternalDeviceId;
    this.#nextExternalDeviceId += 1;
    this.#externalDevices = [...this.#externalDevices, { deviceId, enabled: true, ...spec }];
    this.#publishExternalDevices();
    return deviceId;
  }

  async getBondedBluetoothDevices(): Promise<BondedBluetoothDevices> {
    return this.#bondedBluetoothDevices;
  }

  async editExternalDevice(deviceId: ExternalDeviceId, spec: ConnectionSpec): Promise<void> {
    let index = this.#externalDevices.findIndex((device) => device.deviceId === deviceId);
    if (index === -1) throw unknownExternalDeviceError(deviceId);

    let current = this.#externalDevices[index];
    if (hasConnectionSpec(current, spec)) return;

    let replacement: PublishedExternalDevice = {
      deviceId,
      enabled: current.enabled,
      ...spec,
    };
    this.#externalDevices = this.#externalDevices.map((device, deviceIndex) =>
      deviceIndex === index ? replacement : device,
    );
    this.#publishExternalDevices();
  }

  async setExternalDeviceEnabled(deviceId: ExternalDeviceId, enabled: boolean): Promise<void> {
    let index = this.#externalDevices.findIndex((device) => device.deviceId === deviceId);
    if (index === -1) throw unknownExternalDeviceError(deviceId);

    let current = this.#externalDevices[index];
    if (current.enabled === enabled) return;

    this.#externalDevices = this.#externalDevices.map((device, deviceIndex) =>
      deviceIndex === index ? { ...current, enabled } : device,
    );
    this.#publishExternalDevices();
  }

  async deleteExternalDevice(deviceId: ExternalDeviceId): Promise<void> {
    let index = this.#externalDevices.findIndex((device) => device.deviceId === deviceId);
    if (index === -1) throw unknownExternalDeviceError(deviceId);

    this.#externalDevices = this.#externalDevices.filter((device) => device.deviceId !== deviceId);
    this.#publishExternalDevices();
  }

  async setLocale(locale: Locale): Promise<void> {
    if (this.#settings.locale === locale) return;

    this.#settings = { ...this.#settings, locale };
    this.emit({ topic: 'settings', value: this.#settings });
  }

  async setUnits(units: UnitSettings): Promise<void> {
    let current = this.#settings.units;
    if (
      current.altitude === units.altitude &&
      current.distance === units.distance &&
      current.speed === units.speed &&
      current.verticalSpeed === units.verticalSpeed
    ) {
      return;
    }

    this.#settings = { ...this.#settings, units: { ...units } };
    this.emit({ topic: 'settings', value: this.#settings });
  }

  /** Publishes a topic as though the core had emitted it. */
  emit(topic: Topic): void {
    for (let listener of this.#listeners) {
      listener(topic);
    }
  }

  #publishExternalDevices(): void {
    this.emit({ topic: 'externalDevices', value: this.#externalDevices });
  }
}
