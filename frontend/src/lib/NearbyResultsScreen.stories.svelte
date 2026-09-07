<script module lang="ts">
  import type { Map as MapLibreMap } from 'maplibre-gl';
  import type { AirspaceStore } from './stores/airspace.svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import NearbyAirspaces from '../routes/nearby/[latitude]/[longitude]/NearbyAirspaces.svelte';
  import NearbyTraffic from '../routes/nearby/[latitude]/[longitude]/NearbyTraffic.svelte';
  import NearbyWaypoints from '../routes/nearby/[latitude]/[longitude]/NearbyWaypoints.svelte';
  import NearbyResultsScreen from './NearbyResultsScreen.svelte';
  import { TrafficStore } from './stores/traffic.svelte';

  const position = { latitudeDegrees: 50.82341, longitudeDegrees: 6.18604 };
  const airspace = { current: { generation: 1, sources: [{ type: 'active' }] } } as AirspaceStore;
  const trafficStore = new TrafficStore();
  trafficStore.apply({
    topic: 'traffic',
    value: {
      type: 'snapshot',
      value: [
        {
          id: 'flarm:ABC123',
          position,
          altitudeMslMeters: 1200,
          trafficType: 'glider',
          trackDegrees: 241,
          alarmLevel: 'none',
          stale: false,
        },
      ],
    },
  });

  function previewMap(features: unknown[]): MapLibreMap {
    return {
      on() {},
      off() {},
      isStyleLoaded: () => true,
      isSourceLoaded: () => true,
      getSource: () => ({}),
      getLayer: () => ({}),
      project: () => ({ x: 0, y: 0 }),
      queryRenderedFeatures: () => features,
    } as unknown as MapLibreMap;
  }

  const availableOwnshipRelation = {
    distance: { value: '4.2', unit: 'km' },
    bearing: { value: '063', unit: '°' },
  };

  const summary = {
    arrivalHeight: { stale: true, value: '—' },
    requiredGlideRatio: { stale: true, value: '—' },
    terrainElevation: { stale: true, value: '—' },
  };

  const { Story } = defineMeta({
    title: 'Screens/Nearby results',
    component: NearbyResultsScreen,
    parameters: {
      layout: 'fullscreen',
      docs: {
        description: {
          component:
            'Use this screen after a map tap. The coordinate identifies the selected point without competing with the flight values. Distance and bearing lead the summary. Arrival height, required glide ratio, and terrain elevation remain visible as unknown values until the backend can calculate them. A missing ownship position keeps every dependent value in place and adds a short explanation. Airspace and traffic content use snippets because their asynchronous states and navigating row designs are owned separately. Result lists span the screen at widths up to 34rem and become inset cards above it. The summary remains inset.',
        },
      },
    },
  });
</script>

{#snippet populatedAirspaces()}
  <NearbyAirspaces
    {airspace}
    locale="en"
    {position}
    map={previewMap([
      { id: '1:0', properties: { name: 'Köln Bonn CTR', type: 4, icaoClass: 3 } },
      { id: '1:1', properties: { name: 'EDKB Segelfluggebiet', type: 21, icaoClass: 8 } },
    ])}
  />
{/snippet}

{#snippet populatedWaypoints()}
  <NearbyWaypoints
    altitudeUnit="m"
    sourceStatus="ready"
    {position}
    map={previewMap([
      {
        properties: {
          id: 'club:0',
          name: 'Bonn Hangelar',
          kind: 2,
          elevationMeters: 60,
          frequency: '118.200',
        },
      },
    ])}
  />
{/snippet}

{#snippet populatedTraffic()}
  <NearbyTraffic
    locale="en"
    {position}
    traffic={trafficStore}
    ownship={null}
    units={{ altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' }}
    map={previewMap([{ id: 'flarm:ABC123' }])}
  />
{/snippet}

{#snippet emptyAirspaces()}
  <NearbyAirspaces {airspace} locale="en" {position} map={previewMap([])} />
{/snippet}

{#snippet emptyWaypoints()}
  <NearbyWaypoints altitudeUnit="m" sourceStatus="ready" {position} map={previewMap([])} />
{/snippet}

{#snippet emptyTraffic()}
  <NearbyTraffic
    locale="en"
    {position}
    traffic={trafficStore}
    ownship={null}
    units={{ altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' }}
    map={previewMap([])}
  />
{/snippet}

<Story name="Available position" asChild>
  <div class="nearby-results-story">
    <NearbyResultsScreen
      airspaces={populatedAirspaces}
      waypoints={populatedWaypoints}
      backLabel="Back to map"
      ownshipRelation={availableOwnshipRelation}
      position={{ latitudeDegrees: 50.82341, longitudeDegrees: 6.18604 }}
      {summary}
      title="Nearby"
      traffic={populatedTraffic}
    />
  </div>
</Story>

<Story name="No ownship position" asChild>
  <div class="nearby-results-story">
    <NearbyResultsScreen
      airspaces={emptyAirspaces}
      waypoints={emptyWaypoints}
      backLabel="Back to map"
      ownshipRelation={null}
      position={{ latitudeDegrees: 50.79118, longitudeDegrees: 6.44052 }}
      {summary}
      title="Nearby"
      traffic={emptyTraffic}
    />
  </div>
</Story>

<style>
  .nearby-results-story {
    height: 100vh;
  }
</style>
