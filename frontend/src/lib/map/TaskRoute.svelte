<script lang="ts">
  import type { Task } from '$lib/protocol/generated/Task';

  import { GeoJSONSource, LineLayer } from 'svelte-maplibre-gl';

  type Props = { task: Task };
  let { task }: Props = $props();
  const coordinates = $derived(
    task.points.flatMap(({ target }) =>
      target.type === 'waypoint' ? [[target.longitudeDegrees, target.latitudeDegrees]] : [],
    ),
  );
</script>

{#if coordinates.length >= 2}
  <GeoJSONSource id="task-route" data={{ type: 'LineString', coordinates }}>
    <LineLayer id="task-route-line" paint={{ 'line-color': '#6366f1', 'line-width': 3 }} />
  </GeoJSONSource>
{/if}
