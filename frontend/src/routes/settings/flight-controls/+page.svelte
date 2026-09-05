<script lang="ts">
  import { getAppContext } from '$lib/app-context';
  import GlidePerformanceControls from '$lib/GlidePerformanceControls.svelte';

  const { client, settings, glidePerformance } = getAppContext();
</script>

<GlidePerformanceControls
  macCready={glidePerformance.current.macCready}
  bugs={glidePerformance.current.bugs}
  setBugs={(value) => client.setBugs(value)}
  ballast={glidePerformance.current.ballast}
  setBallast={(value) => client.setBallast(value)}
  unit={settings.current.units.verticalSpeed}
  setMacCready={(value) => client.setMacCready(value)}
/>
