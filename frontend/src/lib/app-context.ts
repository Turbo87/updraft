import type { UpdraftClient } from '#lib/client/index.js';
import type { MapState } from '#lib/map-state.svelte.js';
import type { AirspaceStore } from '#lib/stores/airspace.svelte.js';
import type { BasemapsStore } from '#lib/stores/basemaps.svelte.js';
import type { DataActivation } from '#lib/stores/data-activation.svelte.js';
import type { EnrouteCatalogStore } from '#lib/stores/enroute-catalog.svelte.js';
import type { EnrouteDownloadsStore } from '#lib/stores/enroute-downloads.svelte.js';
import type { ExternalDevicesStore } from '#lib/stores/external-devices.svelte.js';
import type { GlidePerformanceStore } from '#lib/stores/glide-performance.svelte.js';
import type { InstrumentsStore } from '#lib/stores/instruments.svelte.js';
import type { NavigationStore } from '#lib/stores/navigation.svelte.js';
import type { SettingsStore } from '#lib/stores/settings.svelte.js';
import type { TerrainStore } from '#lib/stores/terrain.svelte.js';
import type { TrafficStore } from '#lib/stores/traffic.svelte.js';
import type { WaypointsStore } from '#lib/stores/waypoints.svelte.js';

import { createContext } from 'svelte';

export type AppContext = {
  navigation: NavigationStore;
  client: UpdraftClient;
  airspace: AirspaceStore;
  basemaps: BasemapsStore;
  terrain: TerrainStore;
  enrouteCatalog: EnrouteCatalogStore;
  enrouteDownloads: EnrouteDownloadsStore;
  dataActivation: DataActivation;
  waypoints: WaypointsStore;
  externalDevices: ExternalDevicesStore;
  instruments: InstrumentsStore;
  mapState: MapState;
  settings: SettingsStore;
  glidePerformance: GlidePerformanceStore;
  traffic: TrafficStore;
};

export const [getAppContext, setAppContext] = createContext<AppContext>();
