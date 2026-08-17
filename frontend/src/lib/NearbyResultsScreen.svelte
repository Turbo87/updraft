<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { LatLon } from '$lib/protocol/generated/LatLon';

  import { m } from '$lib/paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';
  import ValueTile from './ValueTile.svelte';

  type Value = {
    value: string;
    unit?: string;
    stale?: boolean;
  };

  type Props = {
    title: string;
    backLabel: string;
    position: LatLon;
    ownshipRelation: {
      distance: Value;
      bearing: Value;
    } | null;
    summary: {
      arrivalHeight: Value;
      requiredGlideRatio: Value;
      terrainElevation: Value;
    };
    airspaces: Snippet;
    traffic: Snippet;
  };

  const sectionId = $props.id();

  let { title, backLabel, position, ownshipRelation, summary, airspaces, traffic }: Props =
    $props();

  const coordinate = $derived(formatCoordinate(position));

  function formatCoordinate({ latitudeDegrees, longitudeDegrees }: LatLon): string {
    let latitudeHemisphere = latitudeDegrees < 0 ? 'S' : 'N';
    let longitudeHemisphere = longitudeDegrees < 0 ? 'W' : 'E';

    return `${Math.abs(latitudeDegrees).toFixed(5)}° ${latitudeHemisphere}, ${Math.abs(longitudeDegrees).toFixed(5)}° ${longitudeHemisphere}`;
  }
</script>

<ScreenScaffold backHref="/" {backLabel} {title}>
  <p class="coordinate">
    <span aria-hidden="true" class="i-mdi-map-marker-outline"></span>
    <span>{coordinate}</span>
  </p>

  <div class="summary">
    <div class="summary-card">
      <div class="primary-values">
        {#if ownshipRelation}
          <ValueTile
            {...ownshipRelation.distance}
            --value-tile-value-font="var(--text-value-md)"
            class="summary-value"
            label={m.distance_label()}
          />
          <ValueTile
            {...ownshipRelation.bearing}
            --value-tile-value-font="var(--text-value-md)"
            class="summary-value"
            label={m.bearing_label()}
          />
        {:else}
          <ValueTile
            --value-tile-value-font="var(--text-value-md)"
            class="summary-value"
            label={m.distance_label()}
            stale
            value="—"
          />
          <ValueTile
            --value-tile-value-font="var(--text-value-md)"
            class="summary-value"
            label={m.bearing_label()}
            stale
            value="—"
          />
        {/if}
      </div>
      <div class="secondary-values">
        <ValueTile
          {...summary.arrivalHeight}
          --value-tile-unit-size="0.875rem"
          --value-tile-value-font="var(--text-value-sm)"
          class="summary-value"
          label={m.arrival_height_label()}
        />
        <ValueTile
          {...summary.requiredGlideRatio}
          --value-tile-unit-size="0.875rem"
          --value-tile-value-font="var(--text-value-sm)"
          class="summary-value"
          label={m.required_glide_ratio_label()}
        />
        <ValueTile
          {...summary.terrainElevation}
          --value-tile-unit-size="0.875rem"
          --value-tile-value-font="var(--text-value-sm)"
          class="summary-value"
          label={m.terrain_elevation_label()}
        />
      </div>
    </div>
    {#if !ownshipRelation}
      <p class="position-notice">{m.nearby_position_unavailable()}</p>
    {/if}
  </div>

  <section aria-labelledby={`${sectionId}-airspaces`}>
    <h2 id={`${sectionId}-airspaces`}>{m.airspaces_heading()}</h2>
    {@render airspaces()}
  </section>

  <section aria-labelledby={`${sectionId}-traffic`}>
    <h2 id={`${sectionId}-traffic`}>{m.traffic_heading()}</h2>
    {@render traffic()}
  </section>
</ScreenScaffold>

<style>
  .coordinate {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    margin: 0 var(--space-1) var(--space-2);
    color: var(--color-text-muted);
    font: 500 0.9375rem / 1.4 var(--font-numeric);
    font-variant-numeric: tabular-nums;
  }

  .coordinate > :first-child {
    flex: 0 0 auto;
    color: var(--color-text-faint);
    font-size: 1.125rem;
    line-height: 1;
  }

  .summary {
    margin-block-end: var(--space-6);
  }

  .summary-card {
    display: grid;
    overflow: hidden;
    gap: 1px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-separator);
  }

  .primary-values,
  .secondary-values {
    display: grid;
    gap: 1px;
    background: var(--color-separator);
  }

  .primary-values {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .secondary-values {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .primary-values :global(.summary-value) {
    align-items: flex-start;
    padding: var(--space-3) var(--space-4);
  }

  .secondary-values :global(.summary-value) {
    align-items: flex-start;
    padding: var(--space-3);
  }

  .position-notice {
    margin: var(--space-4) var(--space-1) 0;
    color: var(--color-text-muted);
    font: var(--text-body);
  }

  section + section {
    margin-block-start: var(--space-6);
  }

  h2 {
    margin: 0 var(--space-1) var(--space-2);
    color: var(--color-text-muted);
    font: var(--text-section-title);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
</style>
