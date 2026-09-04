import type { UpdraftClient } from '$lib/client';
import type { MapState } from '$lib/map-state.svelte';
import type { AirspaceStore } from '$lib/stores/airspace.svelte';
import type { ExternalDevicesStore } from '$lib/stores/external-devices.svelte';
import type { InstrumentsStore } from '$lib/stores/instruments.svelte';
import type { SettingsStore } from '$lib/stores/settings.svelte';
import type { TrafficStore } from '$lib/stores/traffic.svelte';
import type { WaypointsStore } from '$lib/stores/waypoints.svelte';

import { createContext } from 'svelte';

export type AppContext = {
  client: UpdraftClient;
  airspace: AirspaceStore;
  waypoints: WaypointsStore;
  externalDevices: ExternalDevicesStore;
  instruments: InstrumentsStore;
  mapState: MapState;
  settings: SettingsStore;
  traffic: TrafficStore;
};

export const [getAppContext, setAppContext] = createContext<AppContext>();
