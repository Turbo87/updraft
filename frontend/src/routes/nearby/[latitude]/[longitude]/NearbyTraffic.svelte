<script lang="ts">
  import type { Map } from 'maplibre-gl';
  import type { GpsInstruments } from '$lib/protocol/generated/GpsInstruments';
  import type { LatLon } from '$lib/protocol/generated/LatLon';
  import type { Locale } from '$lib/protocol/generated/Locale';
  import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';
  import type { TrafficUpdate } from '$lib/protocol/generated/TrafficUpdate';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';
  import type { RetainedTraffic } from './nearby-traffic';

  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';

  import { calculateDistanceAndBearing } from '$lib/geographic-position';
  import { m } from '$lib/paraglide/messages.js';
  import TrafficSymbol from '$lib/TrafficSymbol.svelte';
  import { convertAltitude, convertDistance } from '$lib/units';
  import {
    createRetainedTraffic,
    formatTrafficId,
    formatTrafficType,
    refreshRetainedTraffic,
  } from './nearby-traffic';

  type QueryState = { type: 'loading' } | { type: 'ready'; traffic: RetainedTraffic[] };

  type Props = {
    locale: Locale;
    map: Map;
    ownship: GpsInstruments | null;
    position: LatLon;
    traffic: TrafficStore;
    units: UnitSettings;
  };

  let { locale, map, ownship, position, traffic, units }: Props = $props();
  let queryState = $state.raw<QueryState>({ type: 'loading' });

  function queryTraffic() {
    if (queryState.type === 'ready') return;

    if (
      !map.isStyleLoaded() ||
      !map.getSource('traffic') ||
      !map.isSourceLoaded('traffic') ||
      !map.getLayer('traffic-hit')
    ) {
      return;
    }

    let point = map.project([position.longitudeDegrees, position.latitudeDegrees]);
    let ids = map.queryRenderedFeatures(point, { layers: ['traffic-hit'] }).map(({ id }) => {
      if (typeof id !== 'string') throw new TypeError('Traffic hit feature has no string ID');
      return id;
    });
    queryState = { type: 'ready', traffic: createRetainedTraffic(ids, traffic.current) };
  }

  function handleTrafficUpdate(
    _update: TrafficUpdate,
    currentTargets: ReadonlyMap<string, PublishedTrafficTarget>,
  ) {
    if (queryState.type === 'loading') return;
    queryState = {
      type: 'ready',
      traffic: refreshRetainedTraffic(queryState.traffic, currentTargets),
    };
  }

  function trafficLabel(retained: RetainedTraffic): string {
    let label = retained.target
      ? `${formatTrafficType(retained.target.trafficType, locale)} · ${formatTrafficId(retained.id)}`
      : formatTrafficId(retained.id);
    return retained.available ? label : `${label} · ${m.unavailable_value()}`;
  }

  function trafficDetail(retained: RetainedTraffic): string {
    let target = retained.target;
    if (!target) return '— · — · —';

    let altitude =
      target.altitudeMslMeters === null
        ? '—'
        : `${convertAltitude(target.altitudeMslMeters, units.altitude).toFixed(0)} ${units.altitude} MSL`;
    let relativeAltitude =
      !ownship || target.altitudeMslMeters === null || ownship.altitudeMeters === null
        ? '—'
        : formatSignedAltitude(target.altitudeMslMeters - ownship.altitudeMeters);
    let distance = ownship
      ? `${convertDistance(
          calculateDistanceAndBearing(ownship.position, target.position).distanceMeters,
          units.distance,
        ).toFixed(1)} ${units.distance}`
      : '—';

    return `${altitude} · ${relativeAltitude} · ${target.stale ? m.stale_value() : distance}`;
  }

  function formatSignedAltitude(meters: number): string {
    let altitude = Math.round(convertAltitude(meters, units.altitude));
    let value = altitude > 0 ? `+${altitude}` : altitude < 0 ? `−${Math.abs(altitude)}` : '0';
    return `${value} ${units.altitude}`;
  }

  onMount(() => {
    map.on('styledata', queryTraffic);
    map.on('sourcedata', queryTraffic);
    let unsubscribe = traffic.subscribe(handleTrafficUpdate);
    queryTraffic();

    return () => {
      map.off('styledata', queryTraffic);
      map.off('sourcedata', queryTraffic);
      unsubscribe();
    };
  });
</script>

{#if queryState.type === 'loading'}
  <p class="empty-results">{m.loading_nearby_traffic()}</p>
{:else if queryState.traffic.length === 0}
  <p class="empty-results">{m.no_nearby_traffic()}</p>
{:else}
  <ul class="result-list">
    {#each queryState.traffic as retained (retained)}
      <li>
        <a href={resolve('/traffic/[id]', { id: retained.id })}>
          <TrafficSymbol
            --traffic-symbol-size="2rem"
            alarmLevel={retained.target?.alarmLevel}
            stale={!retained.available || retained.target?.stale}
            trackDegrees={retained.target?.trackDegrees}
            trafficType={retained.target?.trafficType ?? 'unknown'}
          />
          <span class:stale={!retained.available || retained.target?.stale} class="text">
            <span class="name">{trafficLabel(retained)}</span>
            <span class="detail">{trafficDetail(retained)}</span>
          </span>
          <span aria-hidden="true" class="i-mdi-chevron-right chevron"></span>
        </a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .result-list {
    box-sizing: border-box;
    display: grid;
    width: 100%;
    margin: 0;
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
    list-style: none;
  }

  li {
    min-width: 0;
  }

  li + li {
    border-block-start: 1px solid var(--color-separator);
  }

  a {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    min-width: 0;
    min-height: var(--target-flight);
    padding: var(--space-2) var(--space-4) var(--space-2) var(--space-5);
    border: 0;
    border-radius: 0;
    color: var(--color-text);
    text-decoration: none;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  a:active {
    background: var(--color-control-surface-pressed);
  }

  a:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: -2px;
  }

  .text {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
  }

  .name,
  .detail {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name {
    font: var(--text-row-label);
  }

  .detail {
    color: var(--color-text-muted);
    font: var(--text-row-detail);
    font-family: var(--font-numeric);
    font-variant-numeric: tabular-nums;
  }

  .text.stale {
    color: var(--color-value-stale);
  }

  .text.stale .detail {
    color: inherit;
  }

  .chevron {
    flex: 0 0 auto;
    margin-inline-start: auto;
    color: var(--color-text-muted);
    font-size: 1.75rem;
    line-height: 1;
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

  @media (prefers-reduced-motion: reduce) {
    a {
      transition: none;
    }
  }
</style>
