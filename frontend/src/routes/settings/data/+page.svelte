<script lang="ts">
  import { getAppContext } from '$lib/app-context';
  import DataLibrary from '$lib/DataLibrary.svelte';

  const { airspace, waypoints } = getAppContext();
</script>

<DataLibrary airspace={airspace.current} waypoints={waypoints.current} />
