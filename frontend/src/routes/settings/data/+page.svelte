<script lang="ts">
  import type { BasemapStatus } from '$lib/client';

  import { onMount } from 'svelte';
  import { beforeNavigate } from '$app/navigation';

  import { getAppContext } from '$lib/app-context';
  import DataLibrary from '$lib/DataLibrary.svelte';

  const { client, airspace, waypoints, dataActivation } = getAppContext();
  let detailsOpen = $state(false);
  let basemaps = $state.raw<BasemapStatus | null>(null);
  let basemapError = $state(false);

  onMount(() => {
    let subscription = client.subscribeBasemaps(
      (status) => {
        basemaps = status;
      },
      () => {
        basemapError = true;
      },
    );
    return () => {
      void subscription.close().catch((error: unknown) => {
        console.warn('Could not close basemap subscription', error);
      });
    };
  });

  beforeNavigate((navigation) => {
    if (detailsOpen && navigation.type === 'popstate') {
      navigation.cancel();
      detailsOpen = false;
    }
  });
</script>

<DataLibrary
  {basemaps}
  {basemapError}
  importer={client}
  activation={dataActivation}
  airspace={airspace.current}
  waypoints={waypoints.current}
  bind:detailsOpen
  onRemove={(type, name) =>
    type === 'airspace' ? client.removeAirspace(name) : client.removeWaypoints(name)}
/>
