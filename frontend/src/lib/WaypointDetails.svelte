<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { AltitudeUnit } from './units';
  import type { WaypointFeature } from './waypoints';

  import { m } from '$lib/paraglide/messages.js';
  import Button from './Button.svelte';
  import Card from './Card.svelte';
  import ScreenScaffold from './ScreenScaffold.svelte';
  import { convertAltitude } from './units';

  type Props = {
    waypoint: WaypointFeature;
    altitudeUnit: AltitudeUnit;
    onBack: () => void;
    pinAction?: Snippet<[WaypointFeature]>;
    onNavigate?: () => void;
    error?: boolean;
  };

  let { waypoint, altitudeUnit, onBack, onNavigate, pinAction, error = false }: Props = $props();
  const properties = $derived(waypoint.properties);
</script>

{#snippet navigationAction()}<Button onclick={onNavigate}>{m.navigation_waypoint()}</Button>
  {@render pinAction?.(waypoint)}
{/snippet}

<ScreenScaffold
  title={properties.name}
  backLabel={m.waypoint_back()}
  {onBack}
  actions={onNavigate ? navigationAction : undefined}
>
  {#if error}<p role="alert">{m.navigation_failed()}</p>{/if}
  <p>{m.waypoint_type_value({ kind: properties.kind })}</p>
  <div class="operational">
    <Card>
      <dl class="values">
        <div>
          <dt>{m.terrain_elevation_label()}</dt>
          <dd>
            {convertAltitude(properties.elevationMeters, altitudeUnit).toFixed(0)}
            <span>{altitudeUnit} MSL</span>
          </dd>
        </div>
        {#if properties.frequency}
          <div>
            <dt>{m.frequency_label()}</dt>
            <dd>{properties.frequency} <span>MHz</span></dd>
          </div>
        {/if}
      </dl>
    </Card>
    {#if properties.runwayDirection !== undefined || properties.runwayLengthMeters || properties.runwayWidthMeters}
      <Card>
        <dl class="values runway">
          {#if properties.runwayDirection !== undefined}
            <div>
              <dt>{m.waypoint_runway_direction()}</dt>
              <dd>{String(properties.runwayDirection).padStart(3, '0')}°</dd>
            </div>
          {/if}
          {#if properties.runwayLengthMeters}
            <div>
              <dt>{m.waypoint_runway_length()}</dt>
              <dd>{properties.runwayLengthMeters.toFixed(0)} <span>m</span></dd>
            </div>
          {/if}
          {#if properties.runwayWidthMeters}
            <div>
              <dt>{m.waypoint_runway_width()}</dt>
              <dd>{properties.runwayWidthMeters.toFixed(0)} <span>m</span></dd>
            </div>
          {/if}
        </dl>
      </Card>
    {/if}
  </div>
  <dl class="details">
    {#if properties.notes}
      <dt>{m.waypoint_notes()}</dt>
      <dd class="notes">{properties.notes}</dd>
    {/if}
    <dt>{m.waypoint_coordinates()}</dt>
    <dd>
      {waypoint.geometry.coordinates[1].toFixed(5)}°, {waypoint.geometry.coordinates[0].toFixed(5)}°
    </dd>
    <dt>{m.waypoint_source()}</dt>
    <dd>{properties.sourceName}</dd>
  </dl>
</ScreenScaffold>

<style>
  p,
  dd {
    font: var(--text-body);
  }
  dl {
    margin: 0;
  }
  dt {
    color: var(--color-text-muted);
    font: var(--text-caption);
  }
  dd {
    margin: var(--space-1) 0 0;
    overflow-wrap: anywhere;
  }
  .operational {
    display: grid;
    gap: var(--space-3);
  }
  .values {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-4);
    padding: var(--space-4);
  }
  .values > div {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }
  .runway {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
  .values dd {
    font: var(--text-value-sm);
    font-variant-numeric: tabular-nums;
  }
  .values dd span {
    color: var(--color-text-muted);
    font: var(--text-caption);
  }
  .details dt {
    margin-block-start: var(--space-4);
  }
  .notes {
    white-space: pre-wrap;
  }
</style>
