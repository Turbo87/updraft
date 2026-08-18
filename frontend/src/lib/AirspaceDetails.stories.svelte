<script module lang="ts">
  import type * as GeoJSON from 'geojson';
  import type { GeoJSONSource, Map } from 'maplibre-gl';
  import type { ComponentProps } from 'svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { fn } from 'storybook/test';

  import { AIRSPACE_BROWSER_FIXTURE } from '$lib/map/airspace.fixture';
  import AirspaceDetails from '../routes/airspaces/[id]/AirspaceDetails.svelte';

  function createMap({ data, error }: { data?: GeoJSON.FeatureCollection; error?: Error }): Map {
    let source = data
      ? {
          async getData() {
            if (error) throw error;
            return data;
          },
        }
      : undefined;

    return {
      getSource: () => source as GeoJSONSource | undefined,
      isSourceLoaded: () => source !== undefined,
      off: () => {},
      on: () => {},
    } as unknown as Map;
  }

  const loadedMap = createMap({ data: AIRSPACE_BROWSER_FIXTURE });
  type Args = ComponentProps<typeof AirspaceDetails>;

  const { Story } = defineMeta({
    title: 'Screens/Airspace details',
    component: AirspaceDetails,
    args: {
      altitudeUnit: 'm',
      backLabel: 'Back',
      id: 0,
      locale: 'en',
      map: loadedMap,
      onBack: fn(),
    },
    parameters: {
      layout: 'fullscreen',
      docs: {
        description: {
          component:
            'Airspace details query the active MapLibre airspace source by feature ID. The complete state shows classification, vertical limits, countries, communications, activation, operating hours, and remarks when the source provides them. Optional sections stay absent for a minimal feature. Loading remains visible until the source is ready. A failed source offers a retry action, while an unknown feature ID shows the not-found state.',
        },
      },
    },
  });
</script>

{#snippet template(args: Args)}
  <div class="airspace-details-story">
    <AirspaceDetails {...args} />
  </div>
{/snippet}

<Story name="Complete airspace" {template} />

<Story name="Minimal airspace" args={{ id: 1 }} {template} />

<Story name="Loading" args={{ map: createMap({}) }} {template} />

<Story
  name="Load failure"
  args={{ map: createMap({ data: AIRSPACE_BROWSER_FIXTURE, error: new Error('Load failed') }) }}
  {template}
/>

<Story name="Not found" args={{ id: 999 }} {template} />

<style>
  .airspace-details-story {
    height: 100vh;
  }
</style>
