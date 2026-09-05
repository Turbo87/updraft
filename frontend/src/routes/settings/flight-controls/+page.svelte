<script lang="ts">
  import { getAppContext } from '$lib/app-context';
  import GlidePerformanceControls from '$lib/GlidePerformanceControls.svelte';

  const { client, settings, glidePerformance } = getAppContext();
</script>

<GlidePerformanceControls
  macCready={glidePerformance.current.macCready}
  unit={settings.current.units.verticalSpeed}
  setMacCready={(value) => client.setMacCready(value)}
/>
