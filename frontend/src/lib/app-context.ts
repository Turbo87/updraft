import type { UpdraftClient } from '$lib/client';
import type { SettingsStore } from '$lib/stores/settings.svelte';

import { createContext } from 'svelte';

export type AppContext = {
  client: UpdraftClient;
  settings: SettingsStore;
};

export const [getAppContext, setAppContext] = createContext<AppContext>();
