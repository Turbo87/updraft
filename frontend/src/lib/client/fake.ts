import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';
import type { ChangeSetting } from '$lib/protocol/generated/ChangeSetting';
import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';
import type { ExternalDeviceId } from '$lib/protocol/generated/ExternalDeviceId';
import type { GlidePerformance } from '$lib/protocol/generated/GlidePerformance';
import type { NavigationTarget } from '$lib/protocol/generated/NavigationTarget';
import type { PolarId } from '$lib/protocol/generated/PolarId';
import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';
import type { TaskCommand } from '$lib/protocol/generated/TaskCommand';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { WaypointStatus } from '$lib/protocol/generated/WaypointStatus';
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
  ManagedFileDetails,
  SelectedDataFile,
  TerrainStatus,
  TerrainSubscription,
  TopicListener,
  UpdraftClient,
} from './index';

import { defaultSettings } from '$lib/settings';

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

/**
 * A navigation, pin, or task command call that the fake client records.
 * The fake client does not change state or emit topics for these commands.
 */
export type NavigationCommand =
  | ['saveTask']
  | ['changeTask', TaskCommand]
  | ['pinTarget', NavigationTarget]
  | ['unpinTarget', number]
  | ['setNavigationTarget', NavigationTarget | null];

/** A settings or glide performance command call that the fake client records. */
export type SettingsCommand =
  | ['changeSetting', ChangeSetting]
  | ['setPolar', PolarId]
  | ['setEnergyCompensation', boolean]
  | ['setFlarmPositionCorrection', boolean]
  | ['setMacCready', number]
  | ['setBugs', number]
  | ['setBallast', number];

/** Drives the frontend without a Rust process behind it. */
export class FakeClient implements UpdraftClient {
  readonly settingsCommands: SettingsCommand[] = [];
  readonly navigationCommands: NavigationCommand[] = [];
  #navigationReplies: boolean[] = [];
  /** Sets the results of the next navigation commands. Later commands reply true. */
  queueNavigationReplies(...replies: boolean[]): void {
    this.#navigationReplies.push(...replies);
  }
  #reply(command: NavigationCommand): boolean {
    this.navigationCommands.push(command);
    return this.#navigationReplies.shift() ?? true;
  }
  async saveTask(): Promise<boolean> {
    return this.#reply(['saveTask']);
  }
  async changeTask(command: TaskCommand): Promise<boolean> {
    return this.#reply(['changeTask', command]);
  }
  async pinTarget(target: NavigationTarget): Promise<boolean> {
    return this.#reply(['pinTarget', target]);
  }
  async unpinTarget(id: number): Promise<boolean> {
    return this.#reply(['unpinTarget', id]);
  }
  async setNavigationTarget(target: NavigationTarget | null): Promise<boolean> {
    return this.#reply(['setNavigationTarget', target]);
  }

  #basemaps: BasemapStatus = { generation: 0, sources: [] };
  #basemapListeners = new Set<(status: BasemapStatus) => void>();

  #enrouteCatalog: EnrouteCatalogStatus = { cached: null, refreshing: false, error: false };
  #enrouteCatalogListeners = new Set<(status: EnrouteCatalogStatus) => void>();

  #enrouteDownloads: EnrouteDownloadStatus[] = [];
  #enrouteDownloadsListeners = new Set<(status: EnrouteDownloadStatus[]) => void>();

  #terrain: TerrainStatus = { generation: 0, sources: [] };
  #terrainListeners = new Set<(status: TerrainStatus) => void>();

  #arrivalListeners = new Set<(update: ArrivalUpdate) => void>();
  #glidePerformance: GlidePerformance = { macCready: 0, bugs: 0, ballast: 0 };
  #airspace: AirspaceStatus = { generation: 0, sources: [] };
  #waypoints: WaypointStatus = { generation: 0, sources: [] };
  #airspaceFixtures = new Map<string, AirspaceStatus['sources'][number]>();
  #waypointFixtures = new Map<string, WaypointStatus['sources'][number]>();
  #listeners = new Set<TopicListener>();
  #snapshots = new Map<Topic['topic'], Topic>();
  #externalDevices: PublishedExternalDevice[];
  #nextExternalDeviceId: ExternalDeviceId;
  #bondedBluetoothDevices: BondedBluetoothDevices;
  #settings = defaultSettings();

  constructor(options: FakeClientOptions = {}) {
    this.#externalDevices = options.externalDevices?.map((device) => ({ ...device })) ?? [];
    this.#nextExternalDeviceId = this.#externalDevices.reduce(
      (nextId, device) => Math.max(nextId, device.deviceId + 1),
      1,
    );
    this.#bondedBluetoothDevices = options.bondedBluetoothDevices ?? { status: 'unsupported' };
    let onboarding: Topic[] = [
      { topic: 'navigation', value: null },
      { topic: 'recentTargets', value: [] },
      {
        topic: 'task',
        value: {
          points: [],
          current: null,
          status: 'stopped',
          nextId: 0,
          start: null,
          finish: null,
          restartAllowed: false,
        },
      },
      { topic: 'taskSaveFailed', value: false },
      { topic: 'pinnedTargets', value: [] },
      { topic: 'settings', value: this.#settings },
      { topic: 'glidePerformance', value: this.#glidePerformance },
      { topic: 'externalDevices', value: this.#externalDevices },
      { topic: 'airspace', value: this.#airspace },
      { topic: 'waypoints', value: this.#waypoints },
    ];
    for (let topic of onboarding) this.#snapshots.set(topic.topic, topic);
  }

  subscribeBasemaps(onUpdate: (status: BasemapStatus) => void): BasemapSubscription {
    return subscribeStatus(this.#basemaps, this.#basemapListeners, onUpdate);
  }

  emitBasemaps(status: BasemapStatus): void {
    this.#basemaps = status;
    for (let listener of this.#basemapListeners) listener(status);
  }

  subscribeEnrouteCatalog(
    onUpdate: (status: EnrouteCatalogStatus) => void,
  ): EnrouteCatalogSubscription {
    return subscribeStatus(this.#enrouteCatalog, this.#enrouteCatalogListeners, onUpdate);
  }

  emitEnrouteCatalog(status: EnrouteCatalogStatus): void {
    this.#enrouteCatalog = status;
    for (let listener of this.#enrouteCatalogListeners) listener(status);
  }

  subscribeEnrouteDownloads(
    onUpdate: (status: EnrouteDownloadStatus[]) => void,
  ): EnrouteDownloadSubscription {
    return subscribeStatus(this.#enrouteDownloads, this.#enrouteDownloadsListeners, onUpdate);
  }

  emitEnrouteDownloads(status: EnrouteDownloadStatus[]): void {
    this.#enrouteDownloads = status;
    for (let listener of this.#enrouteDownloadsListeners) listener(status);
  }

  /** Tests and stories supply queue outcomes through emitEnrouteDownloads(). */
  async downloadEnrouteFiles(): Promise<void> {}

  /** Tests and stories supply cancellation outcomes through emitEnrouteDownloads(). */
  async cancelEnrouteDownload(): Promise<void> {}

  async getEnrouteBasemapUpdates(): Promise<string[]> {
    return [];
  }

  async getEnrouteTerrainUpdates(): Promise<string[]> {
    return [];
  }

  async getBasemapFileDetails(): Promise<ManagedFileDetails> {
    throw new Error('Installed file metadata is unavailable in the preview');
  }

  async getTerrainFileDetails(): Promise<ManagedFileDetails> {
    throw new Error('Installed file metadata is unavailable in the preview');
  }

  /** Tests and stories supply refresh outcomes through emitEnrouteCatalog(). */
  async refreshEnrouteCatalog(): Promise<void> {}

  subscribeTerrain(onUpdate: (status: TerrainStatus) => void): TerrainSubscription {
    return subscribeStatus(this.#terrain, this.#terrainListeners, onUpdate);
  }

  emitTerrain(status: TerrainStatus): void {
    this.#terrain = status;
    for (let listener of this.#terrainListeners) listener(status);
  }

  async setBasemapEnabled(sourceName: string, enabled: boolean): Promise<void> {
    let source = this.#basemaps.sources.find((source) => source.sourceName === sourceName);
    if (!source) throw new Error('Basemap file is not installed');
    this.emitBasemaps({
      generation: this.#basemaps.generation + 1,
      sources: this.#basemaps.sources.map((source) =>
        source.sourceName === sourceName
          ? { ...source, type: enabled ? 'active' : 'disabled' }
          : source,
      ),
    });
  }

  async setTerrainEnabled(sourceName: string, enabled: boolean): Promise<void> {
    let source = this.#terrain.sources.find((source) => source.sourceName === sourceName);
    if (!source) throw new Error('Terrain file is not installed');
    this.emitTerrain({
      generation: this.#terrain.generation + 1,
      sources: this.#terrain.sources.map((source) =>
        source.sourceName === sourceName
          ? { ...source, type: enabled ? 'active' : 'disabled' }
          : source,
      ),
    });
  }

  async removeBasemap(sourceName: string): Promise<void> {
    this.emitBasemaps({
      generation: this.#basemaps.generation + 1,
      sources: this.#basemaps.sources.filter((source) => source.sourceName !== sourceName),
    });
  }

  async removeTerrain(sourceName: string): Promise<void> {
    this.emitTerrain({
      generation: this.#terrain.generation + 1,
      sources: this.#terrain.sources.filter((source) => source.sourceName !== sourceName),
    });
  }

  subscribeArrivals(
    _bounds: ArrivalViewport,
    onUpdate: (update: ArrivalUpdate) => void,
  ): ArrivalSubscription {
    this.#arrivalListeners.add(onUpdate);
    return {
      async updateViewport() {},
      close: async () => {
        this.#arrivalListeners.delete(onUpdate);
      },
    };
  }

  /** Supplies a prepared arrival resource for browser development and tests. */
  emitArrivals(update: ArrivalUpdate): void {
    for (let listener of this.#arrivalListeners) listener(update);
  }

  async selectDataFile(): Promise<SelectedDataFile | null> {
    return null;
  }

  async importDataFile(): Promise<SelectedDataFile> {
    throw new Error('No selected data file');
  }

  async discardDataFile(): Promise<void> {}

  async removeWaypoints(sourceName: string): Promise<void> {
    this.emit({
      topic: 'waypoints',
      value: {
        generation: this.#waypoints.generation + 1,
        sources: this.#waypoints.sources.filter((source) => source.sourceName !== sourceName),
      },
    });
  }

  async setWaypointsEnabled(sourceName: string, enabled: boolean): Promise<void> {
    this.emit({
      topic: 'waypoints',
      value: {
        generation: this.#waypoints.generation + 1,
        sources: setSourceEnabled(
          this.#waypoints.sources,
          this.#waypointFixtures,
          sourceName,
          enabled,
        ),
      },
    });
  }

  async setAirspaceEnabled(sourceName: string, enabled: boolean): Promise<void> {
    this.emit({
      topic: 'airspace',
      value: {
        generation: this.#airspace.generation + 1,
        sources: setSourceEnabled(
          this.#airspace.sources,
          this.#airspaceFixtures,
          sourceName,
          enabled,
        ),
      },
    });
  }

  async removeAirspace(sourceName: string): Promise<void> {
    this.emit({
      topic: 'airspace',
      value: {
        generation: this.#airspace.generation + 1,
        sources: this.#airspace.sources.filter((source) => source.sourceName !== sourceName),
      },
    });
  }

  /** Browser development has no session and no process to end. */
  async quit(): Promise<void> {}

  subscribe(onTopic: TopicListener): () => void {
    this.#listeners.add(onTopic);
    for (let topic of this.#snapshots.values()) onTopic(topic);
    onTopic({ topic: 'traffic', value: { type: 'snapshot', value: [] } });

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

  async changeSetting(change: ChangeSetting): Promise<void> {
    this.settingsCommands.push(['changeSetting', change]);
    if (
      change.type === 'arrivalReserve' &&
      (!Number.isFinite(change.reserve) || change.reserve < 0)
    ) {
      throw new Error('Arrival reserve must be finite and nonnegative');
    }
    let settings = this.#settings;
    switch (change.type) {
      case 'locale':
        if (settings.locale === change.locale) return;
        this.#settings = { ...settings, locale: change.locale };
        break;
      case 'units': {
        let current = settings.units;
        let units = change.units;
        if (
          current.altitude === units.altitude &&
          current.distance === units.distance &&
          current.speed === units.speed &&
          current.verticalSpeed === units.verticalSpeed
        )
          return;
        this.#settings = { ...settings, units: { ...units } };
        break;
      }
      case 'arrivalReserve':
        if (settings.arrivalReserve === change.reserve) return;
        this.#settings = { ...settings, arrivalReserve: change.reserve };
        break;
      case 'climbAverageMethod':
        if (settings.climbAverageMethod === change.method) return;
        this.#settings = { ...settings, climbAverageMethod: change.method };
        break;
      case 'hillshadeDirection':
        if (settings.hillshadeDirection === change.direction) return;
        this.#settings = { ...settings, hillshadeDirection: change.direction };
        break;
    }
    this.emit({ topic: 'settings', value: this.#settings });
  }

  async getPolars(): Promise<PolarId[]> {
    return ['LS 8', 'LS 8-18'];
  }

  async setPolar(polar: PolarId): Promise<void> {
    this.settingsCommands.push(['setPolar', polar]);
    if (!(await this.getPolars()).includes(polar)) throw new Error('Unknown polar');
    if (this.#settings.polar === polar) return;
    this.#settings = { ...this.#settings, polar };
    this.emit({ topic: 'settings', value: this.#settings });
  }

  async setEnergyCompensation(enabled: boolean): Promise<void> {
    this.settingsCommands.push(['setEnergyCompensation', enabled]);
    if (this.#settings.energyCompensation === enabled) return;
    this.#settings = { ...this.#settings, energyCompensation: enabled };
    this.emit({ topic: 'settings', value: this.#settings });
  }

  async setFlarmPositionCorrection(enabled: boolean): Promise<void> {
    this.settingsCommands.push(['setFlarmPositionCorrection', enabled]);
    if (this.#settings.flarmPositionCorrection === enabled) return;
    this.#settings = { ...this.#settings, flarmPositionCorrection: enabled };
    this.emit({ topic: 'settings', value: this.#settings });
  }

  async setMacCready(macCready: number): Promise<void> {
    this.settingsCommands.push(['setMacCready', macCready]);
    if (!Number.isFinite(macCready) || macCready < 0) {
      throw new Error('MacCready must be finite and nonnegative');
    }
    if (this.#glidePerformance.macCready === macCready) return;
    this.#glidePerformance = { ...this.#glidePerformance, macCready };
    this.emit({ topic: 'glidePerformance', value: this.#glidePerformance });
  }

  async setBugs(bugs: number): Promise<void> {
    this.settingsCommands.push(['setBugs', bugs]);
    if (!Number.isFinite(bugs) || bugs < 0 || bugs >= 100) {
      throw new Error('Bugs must be between 0% and less than 100%');
    }
    if (this.#glidePerformance.bugs === bugs) return;
    this.#glidePerformance = { ...this.#glidePerformance, bugs };
    this.emit({ topic: 'glidePerformance', value: this.#glidePerformance });
  }

  async setBallast(ballast: number): Promise<void> {
    this.settingsCommands.push(['setBallast', ballast]);
    if (!Number.isFinite(ballast) || ballast < 0) {
      throw new Error('Ballast must be finite and nonnegative');
    }
    if (this.#glidePerformance.ballast === ballast) return;
    this.#glidePerformance = { ...this.#glidePerformance, ballast };
    this.emit({ topic: 'glidePerformance', value: this.#glidePerformance });
  }

  /**
   * Publishes a topic as though the core had emitted it.
   * New subscribers receive the latest snapshot topics.
   * Enabled source records also seed fixtures for later activation.
   */
  emit(topic: Topic): void {
    if (topic.topic !== 'traffic') this.#snapshots.set(topic.topic, topic);
    if (topic.topic === 'airspace') {
      this.#airspace = topic.value;
      updateSourceFixtures(topic.value.sources, this.#airspaceFixtures);
    }
    if (topic.topic === 'waypoints') {
      this.#waypoints = topic.value;
      updateSourceFixtures(topic.value.sources, this.#waypointFixtures);
    }
    for (let listener of this.#listeners) {
      listener(topic);
    }
  }

  #publishExternalDevices(): void {
    this.emit({ topic: 'externalDevices', value: this.#externalDevices });
  }
}

function updateSourceFixtures<T extends { type: string; sourceName: string }>(
  sources: T[],
  fixtures: Map<string, T>,
) {
  for (let name of fixtures.keys()) {
    if (!sources.some((source) => source.sourceName === name)) fixtures.delete(name);
  }
  for (let source of sources) {
    if (source.type !== 'disabled') fixtures.set(source.sourceName, source);
  }
}

function setSourceEnabled<T extends { sourceName: string }>(
  sources: T[],
  fixtures: Map<string, T>,
  sourceName: string,
  enabled: boolean,
) {
  if (!sources.some((source) => source.sourceName === sourceName))
    throw new Error('Source not found');
  let replacement = enabled
    ? (fixtures.get(sourceName) ?? {
        type: 'unavailable' as const,
        sourceName,
        error: 'readFailed' as const,
      })
    : { type: 'disabled' as const, sourceName };
  return sources.map((source) => (source.sourceName === sourceName ? replacement : source));
}

function subscribeStatus<T>(
  status: T,
  listeners: Set<(status: T) => void>,
  onUpdate: (status: T) => void,
): { close(): Promise<void> } {
  onUpdate(status);
  listeners.add(onUpdate);
  return {
    close: async () => {
      listeners.delete(onUpdate);
    },
  };
}
