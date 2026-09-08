<script lang="ts">
  import { beforeNavigate } from '$app/navigation';

  import { getAppContext } from '$lib/app-context';
  import DataLibrary from '$lib/DataLibrary.svelte';

  const { client, airspace, basemaps, terrain, waypoints, dataActivation } = getAppContext();
  let detailsOpen = $state(false);

  beforeNavigate((navigation) => {
    if (detailsOpen && navigation.type === 'popstate') {
      navigation.cancel();
      detailsOpen = false;
    }
  });
</script>

<DataLibrary
  basemaps={basemaps.current}
  basemapError={basemaps.error}
  terrain={terrain.current}
  terrainError={terrain.error}
  importer={client}
  activation={dataActivation}
  airspace={airspace.current}
  waypoints={waypoints.current}
  bind:detailsOpen
  onRemove={(type, name) =>
    type === 'basemap'
      ? client.removeBasemap(name)
      : type === 'airspace'
        ? client.removeAirspace(name)
        : client.removeWaypoints(name)}
/>
