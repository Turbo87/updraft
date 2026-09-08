import type { EnrouteCatalogStatus, UpdraftClient } from '$lib/client';
import type { BasemapsStore } from './basemaps.svelte';

export class EnrouteCatalogStore {
  current = $state.raw<EnrouteCatalogStatus | null>(null);
  error = $state(false);
  basemapUpdates = $state.raw<string[] | null>(null);
  basemapUpdateError = $state(false);

  watchBasemapUpdates(
    client: Pick<UpdraftClient, 'getEnrouteBasemapUpdates'>,
    basemaps: BasemapsStore,
  ) {
    return $effect.root(() => {
      $effect(() => {
        let catalog = this.current;
        let inventory = basemaps.current;
        let failed = this.error || basemaps.error || catalog?.error === true;
        let active = true;
        this.basemapUpdates = null;
        this.basemapUpdateError = failed;
        if (catalog?.cached && !catalog.refreshing && inventory && !basemaps.error) {
          client.getEnrouteBasemapUpdates().then(
            (paths) => {
              if (active) this.basemapUpdates = paths;
            },
            () => {
              if (active) this.basemapUpdateError = true;
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
