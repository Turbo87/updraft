<script module lang="ts">
  import type { PinnedTarget } from '$lib/protocol/generated/PinnedTarget';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import { settingsFixture } from '$lib/settings.fixture';
  import PinnedTargets from './PinnedTargets.svelte';

  const waypoint: PinnedTarget = {
    id: 0,
    primary: false,
    navigation: {
      target: {
        type: 'waypoint',
        name: 'Offenburg',
        latitudeDegrees: 48.45,
        longitudeDegrees: 7.93,
        elevationMeters: 155,
      },
      position: { latitudeDegrees: 48.45, longitudeDegrees: 7.93 },
      guidance: {
        bearingDegrees: 17,
        relativeBearingDegrees: -7,
        distanceMeters: 12600,
        stale: false,
      },
      arrival: { marginMeters: 210, stale: false },
      traffic: null,
    },
  };
  const traffic: PinnedTarget = {
    id: 1,
    primary: false,
    navigation: {
      target: { type: 'traffic', id: 'icao:ABC123' },
      position: { latitudeDegrees: 48.4, longitudeDegrees: 7.9 },
      guidance: {
        bearingDegrees: 42,
        relativeBearingDegrees: 18,
        distanceMeters: 1800,
        stale: true,
      },
      arrival: null,
      traffic: {
        name: 'D-1234',
        ageSeconds: 31,
        stale: true,
        relativeAltitude: { meters: -110, stale: true },
      },
    },
  };
  const map: PinnedTarget = {
    id: 2,
    primary: false,
    navigation: {
      ...waypoint.navigation,
      target: { type: 'mapPosition', latitudeDegrees: 48.5, longitudeDegrees: 8 },
      arrival: null,
    },
  };
  const { Story } = defineMeta({
    title: 'Flight View/Pinned targets',
    component: PinnedTargets,
    args: { pins: [waypoint, traffic, map], units: settingsFixture().units, hasPrimary: false },
  });
</script>

<Story name="Mixed targets" />
<Story
  name="Primary pin hidden"
  args={{ pins: [{ ...waypoint, primary: true }, traffic, map], hasPrimary: true }}
/>
<Story
  name="Waiting for traffic"
  args={{
    pins: [
      {
        ...traffic,
        navigation: { ...traffic.navigation, position: null, traffic: null, guidance: null },
      },
    ],
  }}
/>
<Story
  name="Scrollable list"
  args={{
    pins: Array.from({ length: 12 }, (_, id) => ({
      ...waypoint,
      id,
      navigation: {
        ...waypoint.navigation,
        target: {
          ...waypoint.navigation.target,
          type: 'waypoint',
          name: `Field ${id + 1}`,
          latitudeDegrees: 48.45,
          longitudeDegrees: 7.93,
          elevationMeters: 155,
        },
      },
    })),
  }}
/>

<Story
  name="Task beside standalone goto"
  args={{
    hasPrimary: true,
    pins: [
      { ...waypoint, navigation: { ...waypoint.navigation, target: { type: 'task' } } },
      traffic,
    ],
  }}
/>
