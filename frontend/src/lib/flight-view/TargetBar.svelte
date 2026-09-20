<script lang="ts">
  import type { Navigation } from '$lib/protocol/generated/Navigation';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

  import { resolve } from '$app/paths';

  import { m } from '$lib/paraglide/messages';
  import { getLocale } from '$lib/paraglide/runtime';
  import { convertAltitude, convertDistance } from '$lib/units';

  type Props = { navigation: Navigation; units: UnitSettings };
  let { navigation, units }: Props = $props();
  const guidance = $derived(navigation.guidance);
  const relative = $derived(guidance?.relativeBearingDegrees);
  const altitude = $derived(
    new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 0, signDisplay: 'always' }),
  );
  const number = $derived(new Intl.NumberFormat(getLocale(), { maximumFractionDigits: 1 }));
</script>

<a href={resolve('/navigation')} aria-label={m.navigation_details()} class:stale={guidance?.stale}>
  <strong
    >{navigation.target.type === 'waypoint'
      ? navigation.target.name
      : m.navigation_map_position()}</strong
  >
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
  <span aria-label={m.navigation_arrival()} class:stale={navigation.arrival?.stale}>
    {navigation.arrival
      ? `${altitude.format(convertAltitude(navigation.arrival.marginMeters, units.altitude))} ${units.altitude}`
      : '–'}
  </span>
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
  }
  strong {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  span {
    white-space: nowrap;
  }
  .stale {
    color: var(--color-value-stale);
  }
</style>
