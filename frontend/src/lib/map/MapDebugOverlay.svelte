<script lang="ts">
  import type { Map } from 'maplibre-gl';
  import type { FixTime } from '$lib/protocol/generated/FixTime';
  import type { Instruments } from '$lib/protocol/generated/Instruments';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

  import { convertAltitude, convertSpeed, convertVerticalSpeed } from '$lib/units';

  let {
    map,
    instruments,
    units,
    showTrafficHitAreas = $bindable(false),
  }: {
    map: Map | undefined;
    instruments: Instruments;
    units: UnitSettings;
    showTrafficHitAreas?: boolean;
  } = $props();
  const gps = $derived(instruments.gps);
  const altitudeMeters = $derived(gps?.altitudeMeters ?? null);
  const groundSpeedMetersPerSecond = $derived(gps?.groundSpeedMetersPerSecond ?? null);
  const fixTime = $derived(formatFixTime(gps?.fixTime ?? null));
  const trueAirspeed = $derived(instruments.trueAirspeed);
  const pressureAltitude = $derived(instruments.pressureAltitude);
  const derivedInstruments = $derived(instruments.derived);
  function wind(): string {
    let wind = derivedInstruments?.wind;
    if (!wind) return '–';
    let direction = Math.round(wind.directionDegrees) % 360;
    return `${direction}° / ${speed(wind.speedMetersPerSecond)}`;
  }

  function altitude(meters: number | null | undefined): string {
    return meters === null || meters === undefined
      ? '–'
      : `${convertAltitude(meters, units.altitude).toFixed(0)} ${units.altitude}`;
  }

  function speed(metersPerSecond: number | null | undefined): string {
    return metersPerSecond === null || metersPerSecond === undefined
      ? '–'
      : `${convertSpeed(metersPerSecond, units.speed).toFixed(1)} ${units.speed}`;
  }

  function verticalSpeed(metersPerSecond: number | null | undefined): string {
    return metersPerSecond === null || metersPerSecond === undefined
      ? '–'
      : `${convertVerticalSpeed(metersPerSecond, units.verticalSpeed).toFixed(2)} ${units.verticalSpeed}`;
  }

  function formatFixTime(value: FixTime | null): string {
    if (value === null) return '–';

    let timestamp =
      value.type === 'utcInstant' ? value.unixMilliseconds : value.millisecondsSinceMidnight;
    let iso = new Date(timestamp).toISOString();
    let time = `${iso.slice(11, 23)} UTC`;
    return value.type === 'utcInstant' ? `${iso.slice(0, 10)} ${time}` : time;
  }

  let visible = $state(false);
  let showTileBoundaries = $state(false);

  let zoom = $state(0);
  let lng = $state(0);
  let lat = $state(0);

  function syncView() {
    if (!map) return;
    zoom = map.getZoom();
    let center = map.getCenter();
    lng = center.lng;
    lat = center.lat;
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== 'd' && event.key !== 'D') return;

    let target = event.target as HTMLElement | null;
    if (
      target &&
      (target.isContentEditable ||
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.tagName === 'SELECT')
    ) {
      return;
    }

    visible = !visible;
  }

  // Keep the readout in sync with the map while the overlay is visible.
  $effect(() => {
    let activeMap = map;
    if (!activeMap || !visible) return;
    syncView();
    activeMap.on('move', syncView);
    return () => activeMap.off('move', syncView);
  });

  // MapLibre's tile-boundary debug mode outlines each tile and labels it with
  // its tile ID, which is handy when debugging map rendering.
  $effect(() => {
    if (map) map.showTileBoundaries = showTileBoundaries;
  });
</script>

<svelte:window onkeydown={onKeydown} />

{#if visible}
  <div class="map-debug-overlay">
    <dl>
      <dt>Zoom</dt>
      <dd>{zoom.toFixed(2)}</dd>
      <dt>Center</dt>
      <dd>{lat.toFixed(5)}, {lng.toFixed(5)}</dd>
      <dt>Position</dt>
      <dd class:stale={gps?.stale}>
        {#if gps}
          {gps.position.latitudeDegrees.toFixed(5)},
          {gps.position.longitudeDegrees.toFixed(5)}
        {:else}
          –
        {/if}
      </dd>
      <dt>GPS fix time</dt>
      <dd class:stale={gps?.stale}>{fixTime}</dd>
      <dt>MSL altitude</dt>
      <dd class:stale={gps?.stale}>
        {altitudeMeters === null
          ? '–'
          : `${convertAltitude(altitudeMeters, units.altitude).toFixed(0)} ${units.altitude}`}
      </dd>
      <dt>Ground speed</dt>
      <dd class:stale={gps?.stale}>
        {groundSpeedMetersPerSecond === null
          ? '–'
          : `${convertSpeed(groundSpeedMetersPerSecond, units.speed).toFixed(1)} ${units.speed}`}
      </dd>
      <dt>True airspeed</dt>
      <dd class:stale={trueAirspeed?.stale}>
        {trueAirspeed === null
          ? '–'
          : `${convertSpeed(trueAirspeed.metersPerSecond, units.speed).toFixed(1)} ${units.speed}`}
      </dd>
      <dt>Pressure altitude</dt>
      <dd class:stale={pressureAltitude?.stale}>
        {pressureAltitude === null
          ? '–'
          : `${convertAltitude(pressureAltitude.meters, units.altitude).toFixed(0)} ${units.altitude}`}
      </dd>
      <dt>Raw vertical speed</dt>
      <dd class:stale={derivedInstruments?.rawVerticalSpeed?.stale}>
        {verticalSpeed(derivedInstruments?.rawVerticalSpeed?.metersPerSecond)}
      </dd>
      <dt>Vertical speed</dt>
      <dd class:stale={derivedInstruments?.verticalSpeed?.stale}>
        {verticalSpeed(derivedInstruments?.verticalSpeed?.metersPerSecond)}
      </dd>
      <dt>Vario</dt>
      <dd class:stale={derivedInstruments?.vario?.stale}>
        {verticalSpeed(derivedInstruments?.vario?.metersPerSecond)}
      </dd>
      <dt>Air speed</dt>
      <dd class:stale={derivedInstruments?.airspeed?.stale}>
        {speed(derivedInstruments?.airspeed?.metersPerSecond)}
      </dd>
      <dt>Wind</dt>
      <dd class:stale={derivedInstruments?.wind?.stale}>{wind()}</dd>
      <dt>Derived altitude</dt>
      <dd class:stale={derivedInstruments?.altitude?.stale}>
        {altitude(derivedInstruments?.altitude?.altitudeMslMeters)}
      </dd>
    </dl>
    <label>
      <input type="checkbox" bind:checked={showTileBoundaries} />
      Tile boundaries
    </label>
    <label>
      <input type="checkbox" bind:checked={showTrafficHitAreas} />
      Traffic hit areas
    </label>
  </div>
{/if}

<style>
  .map-debug-overlay {
    position: absolute;
    top: 0.5rem;
    left: 0.5rem;
    z-index: 10;
    padding: 0.5rem 0.75rem;
    border-radius: 0.25rem;
    background: rgb(0 0 0 / 75%);
    color: var(--color-white);
    font-family: monospace;
    font-size: 0.75rem;
    line-height: 1.4;
    pointer-events: auto;
  }

  dl {
    display: grid;
    grid-template-columns: auto auto;
    gap: 0 0.5rem;
    margin: 0 0 0.5rem;
  }

  dt {
    font-weight: bold;
  }

  dd {
    margin: 0;
  }

  dd.stale {
    color: var(--color-value-stale);
  }

  label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    cursor: pointer;
  }
</style>
