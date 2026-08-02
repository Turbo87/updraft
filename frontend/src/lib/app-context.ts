import type { UpdraftClient } from '$lib/client';
import type { ExternalDevicesStore } from '$lib/stores/external-devices.svelte';
import type { SettingsStore } from '$lib/stores/settings.svelte';

import { createContext } from 'svelte';

export type AppContext = {
  client: UpdraftClient;
  externalDevices: ExternalDevicesStore;
  settings: SettingsStore;
};

export const [getAppContext, setAppContext] = createContext<AppContext>();
