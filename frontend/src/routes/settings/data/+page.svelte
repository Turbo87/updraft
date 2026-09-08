<script lang="ts">
  import { beforeNavigate } from '$app/navigation';

  import { getAppContext } from '$lib/app-context';
  import DataLibrary from '$lib/DataLibrary.svelte';

  const { client, airspace, waypoints, dataActivation } = getAppContext();
  let detailsOpen = $state(false);

  beforeNavigate((navigation) => {
    if (detailsOpen && navigation.type === 'popstate') {
      navigation.cancel();
      detailsOpen = false;
    }
  });
</script>

<DataLibrary
  activation={dataActivation}
  airspace={airspace.current}
  waypoints={waypoints.current}
  bind:detailsOpen
  onRemove={(type, name) =>
    type === 'airspace' ? client.removeAirspace(name) : client.removeWaypoints(name)}
/>
