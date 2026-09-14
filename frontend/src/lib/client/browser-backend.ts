import type { Topic } from '$lib/protocol/generated/Topic';
import type { Channel } from '@tauri-apps/api/core';

import { mockConvertFileSrc, mockIPC } from '@tauri-apps/api/mocks';

import { FakeClient } from './fake';

type Subscription = { close(): Promise<void> };

type Command = {
  method: keyof FakeClient;
  arguments?: string[];
};

const commands: Record<string, Command> = {
  add_external_device: { method: 'addExternalDevice', arguments: ['spec'] },
  bonded_bluetooth_devices: { method: 'getBondedBluetoothDevices' },
  cancel_enroute_download: { method: 'cancelEnrouteDownload', arguments: ['path'] },
  delete_external_device: { method: 'deleteExternalDevice', arguments: ['deviceId'] },
  discard_data_file: { method: 'discardDataFile', arguments: ['selectionId'] },
  download_enroute_files: { method: 'downloadEnrouteFiles', arguments: ['paths'] },
  edit_external_device: { method: 'editExternalDevice', arguments: ['deviceId', 'spec'] },
  get_basemap_file_details: { method: 'getBasemapFileDetails', arguments: ['sourceName'] },
  get_enroute_basemap_updates: { method: 'getEnrouteBasemapUpdates' },
  get_enroute_terrain_updates: { method: 'getEnrouteTerrainUpdates' },
  get_polars: { method: 'getPolars' },
  get_terrain_file_details: { method: 'getTerrainFileDetails', arguments: ['sourceName'] },
  import_data_file: { method: 'importDataFile', arguments: ['selectionId'] },
  quit: { method: 'quit' },
  refresh_enroute_catalog: { method: 'refreshEnrouteCatalog' },
  remove_airspace: { method: 'removeAirspace', arguments: ['sourceName'] },
  remove_basemap: { method: 'removeBasemap', arguments: ['sourceName'] },
  remove_terrain: { method: 'removeTerrain', arguments: ['sourceName'] },
  remove_waypoints: { method: 'removeWaypoints', arguments: ['sourceName'] },
  select_data_file: { method: 'selectDataFile' },
  set_airspace_enabled: { method: 'setAirspaceEnabled', arguments: ['sourceName', 'enabled'] },
  set_arrival_reserve: { method: 'setArrivalReserve', arguments: ['reserve'] },
  set_ballast: { method: 'setBallast', arguments: ['ballast'] },
  set_basemap_enabled: { method: 'setBasemapEnabled', arguments: ['sourceName', 'enabled'] },
  set_bugs: { method: 'setBugs', arguments: ['bugs'] },
  set_climb_average_method: { method: 'setClimbAverageMethod', arguments: ['method'] },
  set_energy_compensation: { method: 'setEnergyCompensation', arguments: ['enabled'] },
  set_external_device_enabled: {
    method: 'setExternalDeviceEnabled',
    arguments: ['deviceId', 'enabled'],
  },
  set_flarm_position_correction: {
    method: 'setFlarmPositionCorrection',
    arguments: ['enabled'],
  },
  set_locale: { method: 'setLocale', arguments: ['locale'] },
  set_mac_cready: { method: 'setMacCready', arguments: ['macCready'] },
  set_polar: { method: 'setPolar', arguments: ['polar'] },
  set_terrain_enabled: { method: 'setTerrainEnabled', arguments: ['sourceName', 'enabled'] },
  set_units: { method: 'setUnits', arguments: ['units'] },
  set_waypoints_enabled: { method: 'setWaypointsEnabled', arguments: ['sourceName', 'enabled'] },
};

type ChannelArguments<T> = { channel: Channel<T> };

/** Provides backend state behind the same Tauri IPC client that production uses. */
export class BrowserBackend extends FakeClient {
  #subscriptions = new Map<number, Subscription>();

  install(): void {
    mockConvertFileSrc('linux');
    mockIPC((command, payload = {}) => {
      if (
        Array.isArray(payload) ||
        payload instanceof ArrayBuffer ||
        payload instanceof Uint8Array
      ) {
        throw new Error('Browser backend commands require named arguments');
      }
      return this.#invoke(command, payload);
    });
  }

  #invoke(command: string, payload: Record<string, unknown>): unknown {
    if (command === 'subscribe') {
      let { channel } = payload as ChannelArguments<Topic>;
      this.subscribe((topic) => channel.onmessage(topic));
      return;
    }

    let subscription = this.#subscribe(command, payload);
    if (subscription) return;

    if (command.startsWith('unsubscribe_')) {
      let channelId = payload.channelId as number;
      let current = this.#subscriptions.get(channelId);
      this.#subscriptions.delete(channelId);
      return current?.close();
    }

    let definition = commands[command];
    if (!definition) throw new Error(`Unsupported browser backend command: ${command}`);
    let method = Reflect.get(this, definition.method);
    if (typeof method !== 'function') throw new Error(`Invalid browser backend method: ${definition.method}`);
    let args = definition.arguments?.map((name) => payload[name]) ?? [];
    return Reflect.apply(method, this, args);
  }

  #subscribe(command: string, payload: Record<string, unknown>): Subscription | undefined {
    let method = {
      subscribe_basemaps: 'subscribeBasemaps',
      subscribe_enroute_catalog: 'subscribeEnrouteCatalog',
      subscribe_enroute_downloads: 'subscribeEnrouteDownloads',
      subscribe_terrain: 'subscribeTerrain',
    }[command] as keyof FakeClient | undefined;
    if (!method) return;

    let { channel } = payload as ChannelArguments<unknown>;
    let subscribe = Reflect.get(this, method);
    if (typeof subscribe !== 'function') throw new Error(`Invalid browser subscription: ${method}`);
    let subscription = Reflect.apply(subscribe, this, [
      (message: unknown) => channel.onmessage(message),
    ]) as Subscription;
    this.#subscriptions.set(channel.id, subscription);
    return subscription;
  }
}
