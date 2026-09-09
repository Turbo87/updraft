import type { EnrouteCatalogStatus, UpdraftClient } from '$lib/client';
import type { BasemapsStore } from './basemaps.svelte';
import type { TerrainStore } from './terrain.svelte';

export class EnrouteCatalogStore {
  current = $state.raw<EnrouteCatalogStatus | null>(null);
  error = $state(false);
  updates = $state.raw<string[] | null>(null);
  updateError = $state(false);

  watchUpdates(
    client: Pick<UpdraftClient, 'getEnrouteBasemapUpdates' | 'getEnrouteTerrainUpdates'>,
    basemaps: BasemapsStore,
    terrain: TerrainStore,
  ) {
    return $effect.root(() => {
      $effect(() => {
        let catalog = this.current;
        let ready = basemaps.current && terrain.current && !basemaps.error && !terrain.error;
        let failed = this.error || basemaps.error || terrain.error || catalog?.error === true;
        let active = true;
        this.updates = null;
        this.updateError = failed;
        if (catalog?.cached && !catalog.refreshing && ready) {
          Promise.all([client.getEnrouteBasemapUpdates(), client.getEnrouteTerrainUpdates()]).then(
            (paths) => {
              if (active) this.updates = paths.flat();
            },
            () => {
              if (active) this.updateError = true;
            },
          );
        }
        return () => {
          active = false;
        };
      });
    });
  }
}
