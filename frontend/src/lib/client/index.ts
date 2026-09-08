import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';
import type { ExternalDeviceId } from '$lib/protocol/generated/ExternalDeviceId';
import type { Locale } from '$lib/protocol/generated/Locale';
import type { PolarId } from '$lib/protocol/generated/PolarId';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
import type { BondedBluetoothDevices } from './bonded-bluetooth-devices';

export type TopicListener = (topic: Topic) => void;
/** Unwrapped map bounds: west, south, east, north, with east at least west. */
export type ArrivalViewport = [number, number, number, number];
export type ArrivalUpdate = { generation: number; url: string };
export type ArrivalSubscription = {
  updateViewport(bounds: ArrivalViewport): Promise<void>;
  close(): Promise<void>;
};
export type BasemapStatus = {
  generation: number;
  sources: { sourceName: string; type: 'active' | 'disabled' | 'unavailable' }[];
};
export type BasemapSubscription = { close(): Promise<void> };
export type TerrainStatus = {
  generation: number;
  sources: { sourceName: string; type: 'active' | 'disabled' | 'unavailable' }[];
};
export type TerrainSubscription = { close(): Promise<void> };
export type EnrouteBasemapEntry = {
  path: string;
  countryCode: string;
  continent: 'africa' | 'asia' | 'oceania' | 'europe' | 'northAmerica' | 'southAmerica';
  size: number;
  /** Publication date in YYYY-MM-DD format. */
  publicationDate: string;
};
export type EnrouteCatalogStatus = {
  cached: {
    entries: EnrouteBasemapEntry[];
    /** Unix timestamp in milliseconds. */
    checkedAt: number;
  } | null;
  refreshing: boolean;
  error: boolean;
};
export type EnrouteCatalogSubscription = { close(): Promise<void> };
export type SelectedDataFile = {
  selectionId: string;
  sourceName: string;
  dataType: 'airspace' | 'waypoints';
};

/**
 * The only boundary between the frontend and the Rust shell.
 *
 * Components never import an implementation of this. The layout receives
 * one, so tests and browser-only development can substitute the fake.
 * Mutation promises report command completion. Subscription updates replace
 * shared frontend state.
 */
export interface UpdraftClient {
  /** Delivers the initial inventory and later changes. Reports registration failures through onError. */
  subscribeBasemaps(
    onUpdate: (status: BasemapStatus) => void,
    onError: (error: unknown) => void,
  ): BasemapSubscription;
  /** Delivers terrain inventory snapshots. Reports registration failures through onError. */
  subscribeTerrain(
    onUpdate: (status: TerrainStatus) => void,
    onError: (error: unknown) => void,
  ): TerrainSubscription;
  /** Delivers cached catalog and refresh status. Reports registration failures through onError. */
  subscribeEnrouteCatalog(
    onUpdate: (status: EnrouteCatalogStatus) => void,
    onError: (error: unknown) => void,
  ): EnrouteCatalogSubscription;
  refreshEnrouteCatalog(): Promise<void>;
  /** Reports startup and worker failures through onError. Command promises report their own failures. */
  subscribeArrivals(
    bounds: ArrivalViewport,
    onUpdate: (update: ArrivalUpdate) => void,
    onError: (error: unknown) => void,
  ): ArrivalSubscription;
  /** Adds an enabled external device. */
  addExternalDevice(spec: ConnectionSpec): Promise<ExternalDeviceId>;
  /** Queries the current platform-owned bonded Bluetooth state. */
  getBondedBluetoothDevices(): Promise<BondedBluetoothDevices>;
  /** Replaces one external-device connection specification. */
  editExternalDevice(deviceId: ExternalDeviceId, spec: ConnectionSpec): Promise<void>;
  /** Enables or disables one external device. */
  setExternalDeviceEnabled(deviceId: ExternalDeviceId, enabled: boolean): Promise<void>;
  /** Deletes one configured external device. */
  deleteExternalDevice(deviceId: ExternalDeviceId): Promise<void>;
  /**
   * Starts delivering topics to `onTopic`.
   *
   * The returned function stops local delivery. It does not tell the Rust
   * side to stop sending: the driver prunes a subscriber only when a send
   * to it fails, which happens when the webview goes away. That is enough
   * while the layout owns the only subscription and never unmounts.
   */
  subscribe(onTopic: TopicListener): () => void;
  setLocale(locale: Locale): Promise<void>;
  getPolars(): Promise<PolarId[]>;
  setPolar(polar: PolarId): Promise<void>;
  setArrivalReserve(reserve: number): Promise<void>;
  setMacCready(macCready: number): Promise<void>;
  setBugs(bugs: number): Promise<void>;
  setBallast(ballast: number): Promise<void>;
  /** Replaces all display-unit selections. */
  setUnits(units: UnitSettings): Promise<void>;
  selectDataFile(): Promise<SelectedDataFile | null>;
  importDataFile(selectionId: string): Promise<SelectedDataFile>;
  discardDataFile(selectionId: string): Promise<void>;
  removeWaypoints(sourceName: string): Promise<void>;
  setWaypointsEnabled(sourceName: string, enabled: boolean): Promise<void>;
  removeAirspace(sourceName: string): Promise<void>;
  setAirspaceEnabled(sourceName: string, enabled: boolean): Promise<void>;
  setBasemapEnabled(sourceName: string, enabled: boolean): Promise<void>;
  setTerrainEnabled(sourceName: string, enabled: boolean): Promise<void>;
  removeBasemap(sourceName: string): Promise<void>;
  removeTerrain(sourceName: string): Promise<void>;
  /**
   * Stops the platform session and ends the app.
   *
   * The promise reports that the shell accepted the quit. It does not report
   * that the app ended, because the process goes away underneath it.
   */
  quit(): Promise<void>;
}
