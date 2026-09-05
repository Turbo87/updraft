<script lang="ts">
  import { getAppContext } from '$lib/app-context';
  import WaypointSettings from '$lib/WaypointSettings.svelte';

  const { client, waypoints } = getAppContext();
</script>

<WaypointSettings
  status={waypoints.current}
  onImport={() => client.importWaypoints()}
  onRemove={(name) => client.removeWaypoints(name)}
/>
