<script module lang="ts">
  import type { WaypointFeature } from './waypoints';

  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { fn } from 'storybook/test';

  import WaypointDetails from './WaypointDetails.svelte';

  const waypoint: WaypointFeature = {
    type: 'Feature',
    geometry: { type: 'Point', coordinates: [6, 50] },
    properties: {
      id: '1:0:0',
      sourceName: 'local.cup',
      name: 'Grass airfield',
      kind: 2,
      elevationMeters: 304.8,
      runwayDirection: 90,
      runwayLengthMeters: 800,
      runwayWidthMeters: 30,
      frequency: '123.500',
      notes: 'Circuit north of the airfield.\nCheck local procedures.',
    },
  };
  const { Story } = defineMeta({
    title: 'Components/WaypointDetails',
    component: WaypointDetails,
    args: { waypoint, altitudeUnit: 'm', onBack: fn() },
    parameters: { layout: 'fullscreen' },
  });
</script>

<Story name="Complete" />
<Story name="Feet" args={{ altitudeUnit: 'ft' }} />
<Story
  name="Sparse"
  args={{
    waypoint: {
      ...waypoint,
      properties: {
        id: '1:0:0',
        sourceName: 'peaks.cup',
        name: 'Mountain top',
        kind: 7,
        elevationMeters: 1200,
        frequency: '',
        notes: '',
      },
    },
  }}
/>
