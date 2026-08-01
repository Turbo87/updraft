<script module lang="ts">
  import type { Map } from 'maplibre-gl';
  import type { Instruments } from '$lib/protocol/generated/Instruments';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import MapDebugOverlay from './MapDebugOverlay.svelte';

  const instruments = {
    position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
    altitudeMslMeters: 190,
    trackDegrees: 45,
    groundSpeedMetersPerSecond: 30,
  } satisfies Instruments;

  const map = {
    getZoom: () => 11.25,
    getCenter: () => ({ lng: 6.186, lat: 50.823 }),
    on: () => map,
    off: () => map,
    showTileBoundaries: false,
  } as unknown as Map;

  const { Story } = defineMeta({
    title: 'Map/MapDebugOverlay',
    component: MapDebugOverlay,
  });
</script>

<Story name="Hidden" args={{ map, instruments }} />
<Story
  name="Visible"
  args={{ map, instruments }}
  play={async ({ userEvent }) => {
    await userEvent.keyboard('d');
  }}
/>
