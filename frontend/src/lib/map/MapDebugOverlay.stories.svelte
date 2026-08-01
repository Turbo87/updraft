<script module lang="ts">
  import type { Map } from 'maplibre-gl';
  import type { Instruments } from '$lib/protocol/generated/Instruments';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import MapDebugOverlay from './MapDebugOverlay.svelte';

  const instruments = {
    position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
    altitudeMslMeters: 190,
    trackDegrees: 45,
    groundSpeedMetersPerSecond: 30,
  } satisfies Instruments;

  const metricUnits = {
    altitude: 'm',
    distance: 'km',
    speed: 'km/h',
    verticalSpeed: 'm/s',
  } satisfies UnitSettings;

  const aviationUnits = {
    altitude: 'ft',
    distance: 'nm',
    speed: 'kt',
    verticalSpeed: 'ft/min',
  } satisfies UnitSettings;

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

<Story name="Hidden" args={{ map, instruments, units: metricUnits }} />
<Story
  name="Metric"
  args={{ map, instruments, units: metricUnits }}
  play={async ({ userEvent }) => {
    await userEvent.keyboard('d');
  }}
/>
<Story
  name="Aviation"
  args={{ map, instruments, units: aviationUnits }}
  play={async ({ userEvent }) => {
    await userEvent.keyboard('d');
  }}
/>
