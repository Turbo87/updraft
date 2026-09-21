<script lang="ts">
  import type { Task } from '$lib/protocol/generated/Task';

  import { FillLayer, GeoJSONSource, LineLayer } from 'svelte-maplibre-gl';

  import { taskCylinder } from './task-cylinders';

  type Props = { task: Task };
  let { task }: Props = $props();
  const coordinates = $derived.by(() => {
    let points: number[][] = [];
    for (let { target } of task.points) {
      if (target.type !== 'waypoint') continue;
      let longitude = target.longitudeDegrees;
      let previous = points.at(-1)?.[0];
      if (previous !== undefined) longitude += 360 * Math.round((previous - longitude) / 360);
      points.push([longitude, target.latitudeDegrees]);
    }
    return points;
  });
</script>

{#if coordinates.length}
  <GeoJSONSource
    id="task-cylinders"
    data={{ type: 'MultiPolygon', coordinates: coordinates.map((point) => [taskCylinder(point)]) }}
  >
    <FillLayer id="task-cylinder-fill" paint={{ 'fill-color': '#6366f1', 'fill-opacity': 0.1 }} />
    <LineLayer id="task-cylinder-outline" paint={{ 'line-color': '#6366f1', 'line-width': 2 }} />
  </GeoJSONSource>
{/if}

{#if coordinates.length >= 2}
  <GeoJSONSource id="task-route" data={{ type: 'LineString', coordinates }}>
    <LineLayer id="task-route-line" paint={{ 'line-color': '#6366f1', 'line-width': 3 }} />
  </GeoJSONSource>
{/if}
