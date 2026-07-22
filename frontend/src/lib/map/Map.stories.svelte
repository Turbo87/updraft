<script module lang="ts">
  import type { ComponentProps } from 'svelte';
  import type { GnssState } from '$lib/protocol/generated/GnssState';
  import type { Availability } from '$lib/protocol/generated/Availability';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Map from './Map.svelte';

  const gnss = {
    status: 'current',
    value: {
      position: {
        latitudeDegrees: 50.823,
        longitudeDegrees: 6.186,
      },
      altitudeMeters: 190,
      trackDegrees: 45,
      groundSpeedMetersPerSecond: 30,
    },
  } satisfies Availability<GnssState>;

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

<Story name="No position" args={{ gnss: { status: 'unavailable' } }} {template} />
<Story name="Position" args={{ gnss }} {template} />
<Story name="Test mode" args={{ gnss, testMode: true }} {template} />

<style>
  .map-story {
    width: 100%;
    height: 100vh;
  }
</style>
