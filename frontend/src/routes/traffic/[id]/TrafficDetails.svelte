<script lang="ts">
  import type { AltitudeUnit } from '$lib/protocol/generated/AltitudeUnit';
  import type { DistanceUnit } from '$lib/protocol/generated/DistanceUnit';
  import type { Locale } from '$lib/protocol/generated/Locale';
  import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';
  import type { TrafficUpdate } from '$lib/protocol/generated/TrafficUpdate';
  import type { InstrumentsStore } from '$lib/stores/instruments.svelte';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';

  import { onMount } from 'svelte';

  import { calculateDistanceAndBearing } from '$lib/geographic-position';
  import { m } from '$lib/paraglide/messages.js';
  import { convertAltitude, convertDistance } from '$lib/units';
  import {
    formatTrafficAlarmLevel,
    formatTrafficId,
    formatTrafficType,
  } from '../../nearby/[latitude]/[longitude]/nearby-traffic';

  type RetainedTarget = { target: PublishedTrafficTarget; available: boolean } | null;

  let {
    altitudeUnit,
    distanceUnit,
    id,
    instruments,
    locale,
    traffic,
  }: {
    altitudeUnit: AltitudeUnit;
    distanceUnit: DistanceUnit;
    id: string;
    instruments: InstrumentsStore;
    locale: Locale;
    traffic: TrafficStore;
  } = $props();

  let retainedTarget = $state.raw<RetainedTarget>();
  let ownshipRelation = $derived(
    retainedTarget && instruments.current.gps?.position
      ? calculateDistanceAndBearing(
          instruments.current.gps.position,
          retainedTarget.target.position,
        )
      : null,
  );

  function createRetainedTarget(): RetainedTarget {
    let target = traffic.current.get(id);
    return target ? { target, available: true } : null;
  }

  function handleTrafficUpdate(
    _update: TrafficUpdate,
    currentTargets: ReadonlyMap<string, PublishedTrafficTarget>,
  ) {
    let updatedTarget = currentTargets.get(id);
    if (updatedTarget) {
      retainedTarget = { target: updatedTarget, available: true };
    } else if (retainedTarget) {
      retainedTarget = { ...retainedTarget, available: false };
    }
  }

  onMount(() => {
    retainedTarget = createRetainedTarget();
    return traffic.subscribe(handleTrafficUpdate);
  });
</script>

{#if retainedTarget === undefined}
  <p>{m.traffic_details_loading()}</p>
{:else if retainedTarget === null}
  <p>{m.traffic_not_found()}</p>
{:else}
  {@const target = retainedTarget.target}
  <h1>{formatTrafficId(target.id)}</h1>
  <dl>
    <dt>{m.traffic_id_label()}</dt>
    <dd>{target.id}</dd>
    <dt>{m.traffic_type_label()}</dt>
    <dd>{formatTrafficType(target.trafficType, locale)}</dd>
    <dt>{m.position_label()}</dt>
    <dd>
      {target.position.latitudeDegrees.toFixed(5)}, {target.position.longitudeDegrees.toFixed(5)}
    </dd>
    <dt>{m.altitude_label()}</dt>
    <dd>
      {target.altitudeMslMeters === null
        ? m.unavailable_value()
        : `${convertAltitude(target.altitudeMslMeters, altitudeUnit).toFixed(0)} ${altitudeUnit} MSL`}
    </dd>
    <dt>{m.track_label()}</dt>
    <dd>
      {target.trackDegrees === null
        ? m.unavailable_value()
        : `${target.trackDegrees.toFixed(0)}° true`}
    </dd>
    <dt>{m.alarm_level_label()}</dt>
    <dd>{formatTrafficAlarmLevel(target.alarmLevel, locale)}</dd>
    <dt>{m.state_label()}</dt>
    <dd>
      {retainedTarget.available
        ? target.stale
          ? m.stale_value()
          : m.fresh_value()
        : m.unavailable_value()}
    </dd>
    <dt>{m.distance_label()}</dt>
    <dd>
      {ownshipRelation
        ? `${convertDistance(ownshipRelation.distanceMeters, distanceUnit).toFixed(1)} ${distanceUnit}`
        : m.unavailable_value()}
    </dd>
    <dt>{m.bearing_label()}</dt>
    <dd>
      {ownshipRelation
        ? `${ownshipRelation.bearingDegrees.toFixed(0)}° true`
        : m.unavailable_value()}
    </dd>
  </dl>
{/if}

<style>
  h1 {
    margin-block-start: 0;
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
