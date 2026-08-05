<script lang="ts">
  import { resolve } from '$app/paths';
  import { page } from '$app/state';

  import { getAppContext } from '$lib/app-context';
  import { calculateDistanceAndBearing } from '$lib/geographic-position';
  import { m } from '$lib/paraglide/messages.js';
  import { convertDistance } from '$lib/units';
  import NearbyAirspaces from './NearbyAirspaces.svelte';
  import { parseNearbyRouteCoordinates } from './params';

  const { airspace, instruments, mapState, settings } = getAppContext();
  const selectedPosition = $derived(
    parseNearbyRouteCoordinates(page.params.latitude, page.params.longitude),
  );
  const ownshipRelation = $derived(
    selectedPosition && instruments.current.gps?.position
      ? calculateDistanceAndBearing(instruments.current.gps.position, selectedPosition)
      : null,
  );
  const formattedDistance = $derived(
    ownshipRelation
      ? `${convertDistance(ownshipRelation.distanceMeters, settings.current.units.distance).toFixed(1)} ${settings.current.units.distance}`
      : null,
  );
</script>

<main>
  <a class="back-link" href={resolve('/')}>{m.back_to_map()}</a>

  {#if selectedPosition}
    <h1>{m.nearby_heading()}</h1>
    <dl>
      <dt>{m.selected_position_label()}</dt>
      <dd>
        {selectedPosition.latitudeDegrees.toFixed(5)},
        {selectedPosition.longitudeDegrees.toFixed(5)}
      </dd>
      <dt>{m.distance_label()}</dt>
      <dd>{formattedDistance ?? m.unavailable_value()}</dd>
      <dt>{m.bearing_label()}</dt>
      <dd>
        {ownshipRelation ? `${ownshipRelation.bearingDegrees.toFixed(0)}°` : m.unavailable_value()}
      </dd>
    </dl>

    <section aria-labelledby="airspaces-heading">
      <h2 id="airspaces-heading">{m.airspaces_heading()}</h2>
      {#if airspace.initialized && mapState.map}
        {#key `${selectedPosition.latitudeDegrees}/${selectedPosition.longitudeDegrees}`}
          <NearbyAirspaces {airspace} map={mapState.map} position={selectedPosition} />
        {/key}
      {:else}
        <p>{m.loading_nearby_airspaces()}</p>
      {/if}
    </section>
  {:else}
    <p role="alert">{m.invalid_inspection()}</p>
  {/if}
</main>

<style>
  main {
    box-sizing: border-box;
    min-height: 100%;
    padding: 1.5rem;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  .back-link {
    display: inline-block;
    margin-block-end: 1rem;
  }

  h1 {
    margin-block-start: 0;
  }

  h2 {
    margin-block-end: 0.5rem;
  }

  dl {
    display: grid;
    grid-template-columns: max-content auto;
    gap: 0.5rem 1rem;
  }

  dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
</style>
