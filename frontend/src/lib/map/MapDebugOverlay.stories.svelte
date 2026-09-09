<script module lang="ts">
  import type { Map } from 'maplibre-gl';
  import type { Instruments } from '$lib/protocol/generated/Instruments';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import { EMPTY_DERIVED_INSTRUMENTS, EMPTY_INSTRUMENTS } from '$lib/stores/instruments.svelte';
  import MapDebugOverlay from './MapDebugOverlay.svelte';

  const instruments = {
    gps: {
      position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
      altitudeMeters: 190,
      groundSpeedMetersPerSecond: 30,
      trackDegrees: 45,
      fixTime: { type: 'utcInstant', unixMilliseconds: 1_767_268_800_000 },
      stale: false,
    },
    pressureAltitude: { meters: 1_000, stale: false },
    trueAirspeed: { metersPerSecond: 50, stale: false },
    terrainElevation: { meters: 100, stale: false },
    altitudeAgl: { meters: 110, stale: false },
    derived: {
      ...EMPTY_DERIVED_INSTRUMENTS,
      altitude: { altitudeMslMeters: 210, stale: false },
    },
  } satisfies Instruments;

  const staleInstruments = {
    gps: {
      position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
      altitudeMeters: 190,
      groundSpeedMetersPerSecond: 30,
      trackDegrees: 45,
      fixTime: { type: 'utcTimeOfDay', millisecondsSinceMidnight: 43_201_250 },
      stale: true,
    },
    pressureAltitude: { meters: 1_000, stale: true },
    trueAirspeed: { metersPerSecond: 50, stale: true },
    terrainElevation: { meters: 200, stale: true },
    altitudeAgl: { meters: -10, stale: true },
    derived: {
      ...EMPTY_DERIVED_INSTRUMENTS,
      altitude: { altitudeMslMeters: 190, stale: true },
    },
  } satisfies Instruments;

  const metricUnits = {
    altitude: 'm',
    distance: 'km',
    speed: 'km/h',
    verticalSpeed: 'm/s',
  } satisfies UnitSettings;

  const aviationUnits = {
    altitude: 'ft',
    distance: 'nm',
    speed: 'kt',
    verticalSpeed: 'ft/min',
  } satisfies UnitSettings;

  const map = {
    getZoom: () => 11.25,
    getCenter: () => ({ lng: 6.186, lat: 50.823 }),
    on: () => map,
    off: () => map,
    showTileBoundaries: false,
  } as unknown as Map;

  const { Story } = defineMeta({
    title: 'Map/MapDebugOverlay',
    component: MapDebugOverlay,
  });
</script>

<Story name="Hidden" args={{ map, instruments, units: metricUnits }} />
<Story
  name="Metric"
  args={{ map, instruments, units: metricUnits }}
  play={async ({ userEvent }) => {
    await userEvent.keyboard('d');
  }}
/>
<Story
  name="Aviation"
  args={{ map, instruments: staleInstruments, units: aviationUnits }}
  play={async ({ userEvent }) => {
    await userEvent.keyboard('d');
  }}
/>

<Story
  name="Unavailable"
  args={{ map, instruments: EMPTY_INSTRUMENTS, units: metricUnits }}
  play={async ({ userEvent }) => {
    await userEvent.keyboard('d');
  }}
/>
