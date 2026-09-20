<script lang="ts">
  import type { LatLon } from '$lib/protocol/generated/LatLon';

  import { CircleLayer, GeoJSONSource, LineLayer } from 'svelte-maplibre-gl';

  type Props = { position: LatLon; ownship: LatLon | null };
  let { position, ownship }: Props = $props();
  const destination = $derived([position.longitudeDegrees, position.latitudeDegrees]);
  const courseDestination = $derived([
    position.longitudeDegrees +
      (ownship
        ? 360 * Math.round((ownship.longitudeDegrees - position.longitudeDegrees) / 360)
        : 0),
    position.latitudeDegrees,
  ]);
</script>

<GeoJSONSource id="navigation-target" data={{ type: 'Point', coordinates: destination }}>
  <CircleLayer
    id="navigation-target-marker"
    paint={{
      'circle-radius': 9,
      'circle-color': '#d946ef',
      'circle-stroke-width': 3,
      'circle-stroke-color': '#fff',
    }}
  />
</GeoJSONSource>
{#if ownship}
  <GeoJSONSource
    id="navigation-course"
    data={{
      type: 'LineString',
      coordinates: [[ownship.longitudeDegrees, ownship.latitudeDegrees], courseDestination],
    }}
  >
    <LineLayer
      id="navigation-course-line"
      paint={{ 'line-color': '#d946ef', 'line-width': 3, 'line-dasharray': [3, 2] }}
    />
  </GeoJSONSource>
{/if}
