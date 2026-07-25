<script module lang="ts">
  import type { ComponentProps } from 'svelte';
  import type { Instruments } from '$lib/protocol/generated/Instruments';

  import { defineMeta } from '@storybook/addon-svelte-csf';

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
    instruments: {
      position: null,
      altitudeMslMeters: null,
      trackDegrees: null,
      groundSpeedMetersPerSecond: null,
    },
  }}
  {template}
/>
<Story name="Position" args={{ instruments }} {template} />
<Story name="Test mode" args={{ instruments, testMode: true }} {template} />

<style>
  .map-story {
    width: 100%;
    height: 100vh;
  }
</style>
