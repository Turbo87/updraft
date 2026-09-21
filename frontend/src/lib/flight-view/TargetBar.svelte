<script lang="ts">
  import type { Pathname } from '$app/types';
  import type { Navigation } from '$lib/protocol/generated/Navigation';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

  import { resolve } from '$app/paths';

  import { navigationLabel } from '$lib/navigation';
  import { m } from '$lib/paraglide/messages';
  import { getLocale } from '$lib/paraglide/runtime';
  import { convertAltitude, convertDistance } from '$lib/units';

  type Props = {
    navigation: Navigation;
    units: UnitSettings;
    href?: Pathname;
    label?: string;
    compact?: boolean;
  };
  let {
    navigation,
    units,
    href = '/navigation',
    label = m.navigation_details(),
    compact = false,
  }: Props = $props();
  const guidance = $derived(navigation.guidance);
  const relative = $derived(guidance?.relativeBearingDegrees);
  const altitude = $derived(
    new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 0, signDisplay: 'always' }),
  );
  const number = $derived(new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 1 }));

  function formatReportAge(seconds: number): string {
    if (seconds >= 3600) return `${Math.floor(seconds / 3600)}h`;
    if (seconds >= 60) return `${Math.floor(seconds / 60)}m`;
    return `${Math.floor(seconds)}s`;
  }
</script>

<a href={resolve(href)} aria-label={label} class:compact class:stale={guidance?.stale}>
  {#if compact}<span
      aria-hidden="true"
      class={navigation.target.type === 'task'
        ? 'i-mdi-flag-checkered'
        : navigation.target.type === 'traffic'
          ? 'i-mdi-airplane'
          : navigation.target.type === 'waypoint'
            ? 'i-mdi-map-marker-outline'
            : 'i-mdi-rhombus-outline'}
    ></span>{/if}
  <strong
    >{navigationLabel(navigation)}
    {#if navigation.target.type === 'traffic'}
      {#if !navigation.traffic}<small class="report-age">(n/a)</small>
      {:else if navigation.traffic.stale}<small class="report-age"
          >({formatReportAge(navigation.traffic.ageSeconds)})</small
        >{/if}
    {/if}
  </strong>
  <span aria-label={relative == null ? m.navigation_true() : m.navigation_relative()}>
    {#if guidance}
      {#if relative == null}{Math.round(guidance.bearingDegrees) % 360}° T
      {:else}{relative <= 0 ? '◁' : ''}
        {Math.round(Math.abs(relative))}° {relative >= 0 ? '▷' : ''}{/if}
    {:else}–{/if}
  </span>
  <span
    >{guidance
      ? `${number.format(convertDistance(guidance.distanceMeters, units.distance))} ${units.distance}`
      : '–'}</span
  >
  {#if navigation.target.type === 'traffic'}
    <span
      aria-label={m.navigation_relative_altitude()}
      class:stale={navigation.traffic?.relativeAltitude?.stale}
    >
      {navigation.traffic?.relativeAltitude
        ? `${altitude.format(convertAltitude(navigation.traffic.relativeAltitude.meters, units.altitude))} ${units.altitude}`
        : '–'}
    </span>
  {:else}
    <span aria-label={m.navigation_arrival()} class:stale={navigation.arrival?.stale}>
      {navigation.arrival
        ? `${altitude.format(convertAltitude(navigation.arrival.marginMeters, units.altitude))} ${units.altitude}`
        : '–'}
    </span>
  {/if}
</a>

<style>
  a {
    display: flex;
    gap: var(--space-3);
    align-items: center;
    padding: calc(var(--space-2) + var(--safe-area-top))
      calc(var(--space-3) + var(--safe-area-right)) var(--space-2)
      calc(var(--space-3) + var(--safe-area-left));
    background: var(--color-screen-surface);
    color: var(--color-text);
    text-decoration: none;
    min-height: 3rem;
    border-bottom: 1px solid var(--color-separator);
  }
  a.compact {
    padding-top: var(--space-2);
    font-size: 0.875rem;
    gap: var(--space-2);
  }
  a.compact > span:first-child {
    flex-shrink: 0;
  }
  strong {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  small.report-age {
    display: inline;
    margin-left: var(--space-1);
    font: inherit;
    font-size: smaller;
  }
  span {
    white-space: nowrap;
  }
  .stale {
    color: var(--color-value-stale);
  }
</style>
