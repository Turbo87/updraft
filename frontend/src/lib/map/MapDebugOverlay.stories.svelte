<script module lang="ts">
  import type { Map } from 'maplibre-gl';
  import type { Instruments } from '$lib/protocol/generated/Instruments';
  import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

  import { defineMeta } from '@storybook/addon-svelte-csf';

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
    trueAirspeed: null,
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
    trueAirspeed: null,
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
