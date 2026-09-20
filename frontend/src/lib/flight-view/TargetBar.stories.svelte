<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import { defaultSettings } from '$lib/settings';
  import { waypointNavigation } from './navigation.fixture';
  import TargetBar from './TargetBar.svelte';

  const navigation = waypointNavigation('Home airfield');
  const { Story } = defineMeta({
    title: 'Flight View/TargetBar',
    component: TargetBar,
    args: { navigation, units: defaultSettings().units },
  });
</script>

<Story name="Waypoint" />
<Story name="No position" args={{ navigation: { ...navigation, guidance: null, arrival: null } }} />
<Story
  name="Stale"
  args={{ navigation: { ...navigation, guidance: { ...navigation.guidance, stale: true } } }}
/>
<Story
  name="Waiting for traffic"
  args={{
    navigation: {
      target: { type: 'traffic', id: 'icao:ABC123' },
      position: null,
      guidance: null,
      arrival: null,
      traffic: null,
    },
  }}
/>
<Story
  name="Stale traffic"
  args={{
    navigation: {
      ...navigation,
      target: { type: 'traffic', id: 'icao:ABC123' },
      arrival: null,
      guidance: { ...navigation.guidance, stale: true },
      traffic: {
        name: 'D-TEST',
        ageSeconds: 35,
        stale: true,
        relativeAltitude: { meters: -150, stale: true },
      },
    },
  }}
/>
