<script module lang="ts">
  import type { ComponentProps } from 'svelte';
  import type { GnssData } from '$lib/protocol/generated/GnssData';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Map from './Map.svelte';

  const gnss = {
    position: {
      status: 'current',
      value: {
        latitudeDegrees: 50.823,
        longitudeDegrees: 6.186,
      },
    },
    altitudeMeters: { status: 'current', value: 190 },
    trackDegrees: { status: 'current', value: 45 },
    groundSpeedMetersPerSecond: { status: 'current', value: 30 },
  } satisfies GnssData;

  const { Story } = defineMeta({
    title: 'Map/Map',
    component: Map,
    parameters: { layout: 'fullscreen' },
  });

  type Args = ComponentProps<typeof Map>;
</script>

{#snippet template(args: Args)}
  <div class="map-story">
    <Map {...args} />
  </div>
{/snippet}

<Story
  name="No position"
  args={{
    gnss: {
      position: { status: 'unavailable' },
      altitudeMeters: { status: 'unavailable' },
      trackDegrees: { status: 'unavailable' },
      groundSpeedMetersPerSecond: { status: 'unavailable' },
    },
  }}
  {template}
/>
<Story name="Position" args={{ gnss }} {template} />
<Story name="Test mode" args={{ gnss, testMode: true }} {template} />

<style>
  .map-story {
    width: 100%;
    height: 100vh;
  }
</style>
