import type { ConnectionSpec } from '$lib/protocol/generated/ConnectionSpec';
import type { ExternalDeviceId } from '$lib/protocol/generated/ExternalDeviceId';
import type { Locale } from '$lib/protocol/generated/Locale';
import type { PolarId } from '$lib/protocol/generated/PolarId';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
import type { BondedBluetoothDevices } from './bonded-bluetooth-devices';

export type TopicListener = (topic: Topic) => void;
export type ImportWaypointsResult =
  { type: 'imported'; sourceName: string } | { type: 'cancelled' };
export type ImportAirspaceResult = { type: 'imported' } | { type: 'cancelled' };

/**
 * The only boundary between the frontend and the Rust shell.
 *
 * Components never import an implementation of this. The layout receives
 * one, so tests and browser-only development can substitute the fake.
 * Mutation promises report command completion. Only topics replace shared
 * frontend state.
 */
export interface UpdraftClient {
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
  /** Replaces all display-unit selections. */
  setUnits(units: UnitSettings): Promise<void>;
  importWaypoints(): Promise<ImportWaypointsResult>;
  removeWaypoints(sourceName: string): Promise<void>;
  importAirspace(): Promise<ImportAirspaceResult>;
  removeAirspace(): Promise<void>;
  /**
   * Stops the platform session and ends the app.
   *
   * The promise reports that the shell accepted the quit. It does not report
   * that the app ended, because the process goes away underneath it.
   */
  quit(): Promise<void>;
}
