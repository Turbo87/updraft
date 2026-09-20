<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import { defaultSettings } from '$lib/settings';
  import TargetBar from './TargetBar.svelte';

  const navigation = {
    target: {
      type: 'waypoint' as const,
      name: 'Home airfield',
      latitudeDegrees: 50,
      longitudeDegrees: 6,
      elevationMeters: 100,
    },
    position: { latitudeDegrees: 50, longitudeDegrees: 6 },
    arrival: { marginMeters: 250, stale: false },
    guidance: {
      distanceMeters: 12300,
      bearingDegrees: 90,
      relativeBearingDegrees: -15,
      stale: false,
    },
  };
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
