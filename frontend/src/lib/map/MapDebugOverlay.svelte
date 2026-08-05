<script lang="ts">
  import type { Map } from 'maplibre-gl';
  import type { FixTime } from '$lib/protocol/generated/FixTime';
  import type { Instruments } from '$lib/protocol/generated/Instruments';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

  import { convertAltitude, convertSpeed } from '$lib/units';

  let {
    map,
    instruments,
    units,
  }: { map: Map | undefined; instruments: Instruments; units: UnitSettings } = $props();
  const gps = $derived(instruments.gps);
  const altitudeMeters = $derived(gps?.altitudeMeters ?? null);
  const groundSpeedMetersPerSecond = $derived(gps?.groundSpeedMetersPerSecond ?? null);
  const fixTime = $derived(formatFixTime(gps?.fixTime ?? null));
  const gpsState = $derived(formatDomainState(gps));
  const trueAirspeed = $derived(instruments.trueAirspeed);
  const trueAirspeedState = $derived(formatDomainState(trueAirspeed));
  const pressureAltitude = $derived(instruments.pressureAltitude);
  const pressureAltitudeState = $derived(formatDomainState(pressureAltitude));

  function formatDomainState(value: { stale: boolean } | null): string {
    if (value === null) return 'Unavailable';
    return value.stale ? 'Stale' : 'Current';
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
      {#if gps}
        <dd>
          {gps.position.latitudeDegrees.toFixed(5)},
          {gps.position.longitudeDegrees.toFixed(5)}
        </dd>
      {:else}
        <dd>–</dd>
      {/if}
      <dt>GPS fix time</dt>
      <dd>{fixTime}</dd>
      <dt>GPS state</dt>
      <dd>{gpsState}</dd>
      <dt>MSL altitude</dt>
      <dd>
        {altitudeMeters === null
          ? '–'
          : `${convertAltitude(altitudeMeters, units.altitude).toFixed(0)} ${units.altitude}`}
      </dd>
      <dt>Ground speed</dt>
      <dd>
        {groundSpeedMetersPerSecond === null
          ? '–'
          : `${convertSpeed(groundSpeedMetersPerSecond, units.speed).toFixed(1)} ${units.speed}`}
      </dd>
      <dt>True airspeed</dt>
      <dd>
        {trueAirspeed === null
          ? '–'
          : `${convertSpeed(trueAirspeed.metersPerSecond, units.speed).toFixed(1)} ${units.speed}`}
      </dd>
      <dt>True airspeed state</dt>
      <dd>{trueAirspeedState}</dd>
      <dt>Pressure altitude</dt>
      <dd>
        {pressureAltitude === null
          ? '–'
          : `${convertAltitude(pressureAltitude.meters, units.altitude).toFixed(0)} ${units.altitude}`}
      </dd>
      <dt>Pressure altitude state</dt>
      <dd>{pressureAltitudeState}</dd>
    </dl>
    <label>
      <input type="checkbox" bind:checked={showTileBoundaries} />
      Tile boundaries
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
    background: var(--color-overlay-surface);
    color: var(--color-overlay-text);
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

  label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    cursor: pointer;
  }
</style>
