<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ListRow from './ListRow.svelte';
  import NearbyResultsScreen from './NearbyResultsScreen.svelte';

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
            'Use this screen after a map tap. The coordinate identifies the selected point without competing with the flight values. Distance and bearing lead the summary. Arrival height, required glide ratio, and terrain elevation remain visible as unknown values until the backend can calculate them. A missing ownship position keeps every dependent value in place and adds a short explanation. Airspace and traffic content use snippets because their asynchronous states and navigating row designs are owned separately. Geometry previews are deferred.',
        },
      },
    },
  });
</script>

{#snippet populatedAirspaces()}
  <ul class="result-list">
    <li>
      <ListRow
        href="/airspaces/42"
        label="Köln Bonn CTR"
        size="large"
        value="Control zone · Class D"
      />
    </li>
    <li>
      <ListRow
        href="/airspaces/84"
        label="EDKB Segelfluggebiet"
        size="large"
        value="Gliding sector"
      />
    </li>
  </ul>
{/snippet}

{#snippet populatedTraffic()}
  <ul class="result-list">
    <li>
      <ListRow href="/traffic/DDX7A2" label="Glider · DDX7A2" size="large" value="3.9 km" />
    </li>
    <li>
      <ListRow href="/traffic/ICA3F19" label="Tow plane · ICA3F19" size="large" value="stale" />
    </li>
  </ul>
{/snippet}

{#snippet emptyAirspaces()}
  <p class="empty-results">No airspace at this position.</p>
{/snippet}

{#snippet emptyTraffic()}
  <p class="empty-results">No traffic at this position.</p>
{/snippet}

<Story name="Available position" asChild>
  <div class="nearby-results-story">
    <NearbyResultsScreen
      airspaces={populatedAirspaces}
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

  .result-list {
    display: grid;
    margin: 0;
    padding: 0;
    gap: var(--space-2);
    list-style: none;
  }

  .empty-results {
    margin: 0;
    padding: var(--space-5);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
    color: var(--color-text-muted);
    font: var(--text-body);
  }
</style>
