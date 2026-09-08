import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';
import type { ExternalDeviceId } from '$lib/protocol/generated/ExternalDeviceId';
import type { Locale } from '$lib/protocol/generated/Locale';
import type { PolarId } from '$lib/protocol/generated/PolarId';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
import type { BondedBluetoothDevices } from './bonded-bluetooth-devices';
import type {
  ArrivalSubscription,
  ArrivalUpdate,
  ArrivalViewport,
  BasemapStatus,
  BasemapSubscription,
  EnrouteCatalogStatus,
  EnrouteCatalogSubscription,
  EnrouteDownloadStatus,
  EnrouteDownloadSubscription,
  SelectedDataFile,
  TerrainStatus,
  TerrainSubscription,
  TopicListener,
  UpdraftClient,
} from './index';

import { Channel, convertFileSrc, invoke } from '@tauri-apps/api/core';

type ArrivalNotification =
  { type: 'ready'; generation: number; revision: number } | { type: 'failed' };

/** Invokes the concrete Tauri commands that form the frontend shell boundary. */
export class TauriClient implements UpdraftClient {
  subscribeBasemaps(
    onUpdate: (status: BasemapStatus) => void,
    onError: (error: unknown) => void,
  ): BasemapSubscription {
    let channel = new Channel<BasemapStatus>();
    channel.onmessage = onUpdate;
    let closed = false;
    let closing: Promise<void> | undefined;
    let ready = invoke('subscribe_basemaps', { channel }).then(
      () => true,
      (error: unknown) => {
        channel.onmessage = () => {};
        if (!closed) onError(error);
        return false;
      },
    );
    return {
      close() {
        if (closing) return closing;
        closed = true;
        channel.onmessage = () => {};
        closing = ready.then(async (registered) => {
          if (registered) await invoke('unsubscribe_basemaps', { channelId: channel.id });
        });
        return closing;
      },
    };
  }

  subscribeEnrouteCatalog(
    onUpdate: (status: EnrouteCatalogStatus) => void,
    onError: (error: unknown) => void,
  ): EnrouteCatalogSubscription {
    let channel = new Channel<EnrouteCatalogStatus>();
    channel.onmessage = onUpdate;
    let closed = false;
    let closing: Promise<void> | undefined;
    let ready = invoke('subscribe_enroute_catalog', { channel }).then(
      () => true,
      (error: unknown) => {
        channel.onmessage = () => {};
        if (!closed) onError(error);
        return false;
      },
    );
    return {
      close() {
        if (closing) return closing;
        closed = true;
        channel.onmessage = () => {};
        closing = ready.then(async (registered) => {
          if (registered) await invoke('unsubscribe_enroute_catalog', { channelId: channel.id });
        });
        return closing;
      },
    };
  }

  subscribeEnrouteDownloads(
    onUpdate: (status: EnrouteDownloadStatus[]) => void,
    onError: (error: unknown) => void,
  ): EnrouteDownloadSubscription {
    let channel = new Channel<EnrouteDownloadStatus[]>();
    channel.onmessage = onUpdate;
    let closed = false;
    let closing: Promise<void> | undefined;
    let ready = invoke('subscribe_enroute_downloads', { channel }).then(
      () => true,
      (error: unknown) => {
        channel.onmessage = () => {};
        if (!closed) onError(error);
        return false;
      },
    );
    return {
      close() {
        if (closing) return closing;
        closed = true;
        channel.onmessage = () => {};
        closing = ready.then(async (registered) => {
          if (registered) await invoke('unsubscribe_enroute_downloads', { channelId: channel.id });
        });
        return closing;
      },
    };
  }

  downloadEnrouteBasemaps(paths: string[]): Promise<void> {
    return invoke('download_enroute_basemaps', { paths });
  }

  cancelEnrouteDownload(path: string): Promise<void> {
    return invoke('cancel_enroute_download', { path });
  }

  getEnrouteBasemapUpdates(): Promise<string[]> {
    return invoke('get_enroute_basemap_updates');
  }

  refreshEnrouteCatalog(): Promise<void> {
    return invoke('refresh_enroute_catalog');
  }

  subscribeTerrain(
    onUpdate: (status: TerrainStatus) => void,
    onError: (error: unknown) => void,
  ): TerrainSubscription {
    let channel = new Channel<TerrainStatus>();
    channel.onmessage = onUpdate;
    let closed = false;
    let closing: Promise<void> | undefined;
    let ready = invoke('subscribe_terrain', { channel }).then(
      () => true,
      (error: unknown) => {
        channel.onmessage = () => {};
        if (!closed) onError(error);
        return false;
      },
    );
    return {
      close() {
        if (closing) return closing;
        closed = true;
        channel.onmessage = () => {};
        closing = ready.then(async (registered) => {
          if (registered) await invoke('unsubscribe_terrain', { channelId: channel.id });
        });
        return closing;
      },
    };
  }

  subscribeArrivals(
    bounds: ArrivalViewport,
    onUpdate: (update: ArrivalUpdate) => void,
    onError: (error: unknown) => void,
  ): ArrivalSubscription {
    let channel = new Channel<ArrivalNotification>();
    let id: string | undefined;
    let pending: Extract<ArrivalNotification, { type: 'ready' }> | undefined;
    let closed = false;
    let closing: Promise<void> | undefined;
    function receive(notification: ArrivalNotification) {
      if (closed) return;
      if (notification.type === 'failed') {
        pending = undefined;
        onError(new Error('Arrival worker stopped'));
      } else if (id === undefined) {
        pending = notification;
      } else {
        let url = `${convertFileSrc('arrivals', 'updraft')}/${id}.geojson`;
        onUpdate({ generation: notification.generation, url: `${url}?v=${notification.revision}` });
      }
    }
    channel.onmessage = receive;
    let ready = invoke<string>('start_arrivals', { bounds, channel }).then(
      (subscriptionId) => {
        id = subscriptionId;
        if (pending) receive(pending);
        return id;
      },
      (error: unknown) => {
        if (!closed) onError(error);
        return undefined;
      },
    );
    return {
      async updateViewport(bounds) {
        let id = await ready;
        if (id !== undefined && !closed) await invoke('update_arrival_viewport', { id, bounds });
      },
      close() {
        if (closing) return closing;
        closed = true;
        channel.onmessage = () => {};
        closing = ready.then(async (id) => {
          if (id !== undefined) await invoke('stop_arrivals', { id });
        });
        return closing;
      },
    };
  }

  addExternalDevice(spec: ConnectionSpec): Promise<ExternalDeviceId> {
    return invoke('add_external_device', { spec });
  }

  getBondedBluetoothDevices(): Promise<BondedBluetoothDevices> {
    return invoke('bonded_bluetooth_devices');
  }

  editExternalDevice(deviceId: ExternalDeviceId, spec: ConnectionSpec): Promise<void> {
    return invoke('edit_external_device', { deviceId, spec });
  }

  setExternalDeviceEnabled(deviceId: ExternalDeviceId, enabled: boolean): Promise<void> {
    return invoke('set_external_device_enabled', { deviceId, enabled });
  }

  deleteExternalDevice(deviceId: ExternalDeviceId): Promise<void> {
    return invoke('delete_external_device', { deviceId });
  }

  selectDataFile(): Promise<SelectedDataFile | null> {
    return invoke('select_data_file');
  }

  importDataFile(selectionId: string): Promise<SelectedDataFile> {
    return invoke('import_data_file', { selectionId });
  }

  discardDataFile(selectionId: string): Promise<void> {
    return invoke('discard_data_file', { selectionId });
  }

  removeWaypoints(sourceName: string): Promise<void> {
    return invoke('remove_waypoints', { sourceName });
  }

  setWaypointsEnabled(sourceName: string, enabled: boolean): Promise<void> {
    return invoke('set_waypoints_enabled', { sourceName, enabled });
  }

  removeAirspace(sourceName: string): Promise<void> {
    return invoke('remove_airspace', { sourceName });
  }

  setAirspaceEnabled(sourceName: string, enabled: boolean): Promise<void> {
    return invoke('set_airspace_enabled', { sourceName, enabled });
  }

  setBasemapEnabled(sourceName: string, enabled: boolean): Promise<void> {
    return invoke('set_basemap_enabled', { sourceName, enabled });
  }

  setTerrainEnabled(sourceName: string, enabled: boolean): Promise<void> {
    return invoke('set_terrain_enabled', { sourceName, enabled });
  }

  removeTerrain(sourceName: string): Promise<void> {
    return invoke('remove_terrain', { sourceName });
  }

  removeBasemap(sourceName: string): Promise<void> {
    return invoke('remove_basemap', { sourceName });
  }

  quit(): Promise<void> {
    return invoke('quit');
  }

  setLocale(locale: Locale): Promise<void> {
    return invoke('set_locale', { locale });
  }

  setUnits(units: UnitSettings): Promise<void> {
    return invoke('set_units', { units });
  }

  getPolars(): Promise<PolarId[]> {
    return invoke('get_polars');
  }

  setPolar(polar: PolarId): Promise<void> {
    return invoke('set_polar', { polar });
  }

  setArrivalReserve(reserve: number): Promise<void> {
    return invoke('set_arrival_reserve', { reserve });
  }

  setMacCready(macCready: number): Promise<void> {
    return invoke('set_mac_cready', { macCready });
  }

  setBugs(bugs: number): Promise<void> {
    return invoke('set_bugs', { bugs });
  }

  setBallast(ballast: number): Promise<void> {
    return invoke('set_ballast', { ballast });
  }

  subscribe(onTopic: TopicListener): () => void {
    let channel = new Channel<Topic>();
    channel.onmessage = onTopic;

    let closed = false;
    void invoke('subscribe', { channel }).catch((error: unknown) => {
      if (!closed) {
        console.error('Failed to subscribe to the state stream', error);
      }
    });

    return () => {
      closed = true;
      channel.onmessage = () => {};
    };
  }
}
