<script lang="ts">
  import type { AltitudeUnit } from './units';
  import type { WaypointFeature } from './waypoints';

  import { m } from '$lib/paraglide/messages.js';
  import ScreenScaffold from './ScreenScaffold.svelte';
  import { convertAltitude } from './units';

  let {
    waypoint,
    altitudeUnit,
    onBack,
  }: { waypoint: WaypointFeature; altitudeUnit: AltitudeUnit; onBack: () => void } = $props();
  const properties = $derived(waypoint.properties);
</script>

<ScreenScaffold title={properties.name} backLabel={m.waypoint_back()} {onBack}>
  <p>{m.waypoint_type_value({ kind: properties.kind })}</p>
  <dl>
    <dt>{m.waypoint_coordinates()}</dt>
    <dd>
      {waypoint.geometry.coordinates[1].toFixed(5)}°, {waypoint.geometry.coordinates[0].toFixed(5)}°
    </dd>
    <dt>{m.terrain_elevation_label()}</dt>
    <dd>
      {convertAltitude(properties.elevationMeters, altitudeUnit).toFixed(0)}
      {altitudeUnit} MSL
    </dd>
    {#if properties.runwayDirection !== undefined}
      <dt>{m.waypoint_runway_direction()}</dt>
      <dd>{String(properties.runwayDirection).padStart(3, '0')}°</dd>
    {/if}
    {#if properties.runwayLengthMeters !== undefined}
      <dt>{m.waypoint_runway_length()}</dt>
      <dd>{properties.runwayLengthMeters.toFixed(0)} m</dd>
    {/if}
    {#if properties.runwayWidthMeters !== undefined}
      <dt>{m.waypoint_runway_width()}</dt>
      <dd>{properties.runwayWidthMeters.toFixed(0)} m</dd>
    {/if}
    {#if properties.frequency}
      <dt>{m.frequency_label()}</dt>
      <dd>{properties.frequency}</dd>
    {/if}
    <dt>{m.waypoint_source()}</dt>
    <dd>{properties.sourceName}</dd>
    {#if properties.notes}
      <dt>{m.waypoint_notes()}</dt>
      <dd class="notes">{properties.notes}</dd>
    {/if}
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
    margin-block-start: var(--space-4);
    color: var(--color-text-muted);
    font: var(--text-row-label);
  }
  dd {
    margin: var(--space-1) 0 0;
    overflow-wrap: anywhere;
  }
  .notes {
    white-space: pre-wrap;
  }
</style>
