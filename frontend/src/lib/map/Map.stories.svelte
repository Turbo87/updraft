<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';
  import type { ComponentProps } from 'svelte';
  import type { PositionFix } from '$lib/protocol/generated/PositionFix';
  import Map from './Map.svelte';

  const position = {
    observedAtMs: 0,
    latitudeDegrees: 50.823,
    longitudeDegrees: 6.186,
    altitudeMeters: 190,
    trackDegrees: 45,
    groundSpeedMetersPerSecond: 30,
  } satisfies PositionFix;

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

<Story name="No position" args={{ position: null }} {template} />
<Story name="Position" args={{ position }} {template} />
<Story name="Test mode" args={{ position, testMode: true }} {template} />

<style>
  .map-story {
    width: 100%;
    height: 100vh;
  }
</style>
