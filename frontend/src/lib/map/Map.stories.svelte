<script module lang="ts">
  import type { GeoJSONSource, Map as MapLibreMap } from 'maplibre-gl';
  import type { ComponentProps } from 'svelte';
  import type { Instruments } from '$lib/protocol/generated/Instruments';

  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { expect, waitFor } from 'storybook/test';

  import { TrafficStore } from '$lib/stores/traffic.svelte';
  import Map from './Map.svelte';

  const instruments = {
    position: {
      latitudeDegrees: 50.823,
      longitudeDegrees: 6.186,
    },
    altitudeMslMeters: 190,
    trackDegrees: 45,
    groundSpeedMetersPerSecond: 30,
  } satisfies Instruments;

  const traffic = new TrafficStore();
  traffic.apply({
    topic: 'traffic',
    value: {
      type: 'snapshot',
      value: [
        {
          id: 'flarm:000001',
          position: { latitudeDegrees: 50.826, longitudeDegrees: 6.18 },
          altitudeMslMeters: 350,
          trafficType: 'glider',
          trackDegrees: 45,
          alarmLevel: 'none',
          stale: false,
        },
        {
          id: 'flarm:000002',
          position: { latitudeDegrees: 50.819, longitudeDegrees: 6.19 },
          altitudeMslMeters: 280,
          trafficType: 'paraglider',
          trackDegrees: 225,
          alarmLevel: 'important',
          stale: false,
        },
        {
          id: 'flarm:000003',
          position: { latitudeDegrees: 50.83, longitudeDegrees: 6.2 },
          altitudeMslMeters: 420,
          trafficType: 'balloon',
          trackDegrees: 90,
          alarmLevel: 'low',
          stale: false,
        },
        {
          id: 'flarm:000004',
          position: { latitudeDegrees: 50.814, longitudeDegrees: 6.176 },
          altitudeMslMeters: 230,
          trafficType: 'pistonAircraft',
          trackDegrees: null,
          alarmLevel: 'none',
          stale: false,
        },
        {
          id: 'flarm:000005',
          position: { latitudeDegrees: 50.835, longitudeDegrees: 6.17 },
          altitudeMslMeters: 510,
          trafficType: 'helicopter',
          trackDegrees: 135,
          alarmLevel: 'urgent',
          stale: true,
        },
        {
          id: 'flarm:000006',
          position: { latitudeDegrees: 50.81, longitudeDegrees: 6.205 },
          altitudeMslMeters: null,
          trafficType: 'airship',
          trackDegrees: 315,
          alarmLevel: 'none',
          stale: false,
        },
      ],
    },
  });

  const { Story } = defineMeta({
    title: 'Map/Map',
    component: Map,
    parameters: { layout: 'fullscreen' },
  });

  type Args = ComponentProps<typeof Map>;
  type TestWindow = Window & {
    __updraftTest?: { map: MapLibreMap };
  };
</script>

{#snippet template(args: Args)}
  <div class="map-story">
    <Map {...args} />
  </div>
{/snippet}

<Story
  name="No position"
  args={{
    instruments: {
      position: null,
      altitudeMslMeters: null,
      trackDegrees: null,
      groundSpeedMetersPerSecond: null,
    },
    traffic,
  }}
  {template}
/>
<Story name="Position" args={{ instruments, traffic }} {template} />
<Story
  name="Test mode"
  args={{ instruments, traffic, testMode: true }}
  play={async () => {
    await waitFor(async () => {
      let map = (window as TestWindow).__updraftTest?.map;
      let data = await map?.getSource<GeoJSONSource>('traffic')?.getData();
      let featureCount = data && 'features' in data ? data.features.length : 0;

      expect(featureCount).toBe(6);
    });
  }}
  {template}
/>

<style>
  .map-story {
    width: 100%;
    height: 100vh;
  }
</style>
