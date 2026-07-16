<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';
  import type { Map } from 'maplibre-gl';
  import MapDebugOverlay from './MapDebugOverlay.svelte';

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

<Story name="Hidden" args={{ map }} />
<Story
  name="Visible"
  args={{ map }}
  play={async ({ userEvent }) => {
    await userEvent.keyboard('d');
  }}
/>
