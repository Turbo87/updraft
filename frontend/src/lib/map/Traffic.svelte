<script lang="ts">
  import type {
    ExpressionSpecification,
    GeoJSONSource as MapLibreGeoJSONSource,
    SymbolLayerSpecification,
  } from 'maplibre-gl';
  import type { AltitudeUnit } from '$lib/protocol/generated/AltitudeUnit';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';

  import { onMount } from 'svelte';
  import { CircleLayer, GeoJSONSource, SymbolLayer } from 'svelte-maplibre-gl';

  import { FONT_REGULAR } from './basemap-style';
  import {
    COLOR_AMBER_500,
    COLOR_ORANGE_500,
    COLOR_RED_600,
    COLOR_SLATE_900,
    COLOR_WHITE,
  } from './colors.generated';
  import { applyTrafficSourceUpdate, trafficFeatureCollection } from './traffic';

  type SymbolLayout = NonNullable<SymbolLayerSpecification['layout']>;
  type SymbolPaint = NonNullable<SymbolLayerSpecification['paint']>;

  const IF_STALE: ExpressionSpecification = ['boolean', ['get', 'stale'], false];

  const TRAFFIC_OPACITY: ExpressionSpecification = ['case', IF_STALE, 0.45, 1];

  const TRAFFIC_LAYOUT: SymbolLayout = {
    'icon-image': [
      'match',
      ['get', 'trafficType'],
      'glider',
      'updraft-sdf:glider',
      ['towPlane', 'dropPlane', 'pistonAircraft'],
      'updraft-sdf:aircraft',
      'helicopter',
      'updraft-sdf:helicopter',
      'hangGlider',
      'updraft-sdf:hang-glider',
      'paraglider',
      'updraft-sdf:paraglider',
      'jetAircraft',
      'updraft-sdf:jet',
      'balloon',
      'updraft-sdf:balloon',
      'airship',
      'updraft-sdf:airship',
      'updraft-sdf:unknown',
    ],
    'icon-size': ['interpolate', ['linear'], ['zoom'], 4, 0.3, 8, 0.75],
    'icon-allow-overlap': true,
    'text-field': [
      'step',
      ['zoom'],
      ['coalesce', ''],
      7,
      ['coalesce', ['get', 'altitudeLabel'], ''],
    ],
    'text-font': FONT_REGULAR,
    'text-size': 11,
    'text-rotation-alignment': 'viewport',
    'text-allow-overlap': true,
    'text-offset': [0, 2],
    'symbol-sort-key': ['match', ['get', 'alarmLevel'], 'low', 1, 'important', 2, 'urgent', 3, 0],
  };

  const TRAFFIC_PAINT: SymbolPaint = {
    'icon-color': [
      'match',
      ['get', 'alarmLevel'],
      'low',
      COLOR_AMBER_500,
      'important',
      COLOR_ORANGE_500,
      'urgent',
      COLOR_RED_600,
      COLOR_WHITE,
    ],
    'icon-opacity': TRAFFIC_OPACITY,
    'text-color': COLOR_SLATE_900,
    'text-opacity': TRAFFIC_OPACITY,
    'icon-halo-color': COLOR_SLATE_900,
    'icon-halo-width': ['interpolate', ['linear'], ['zoom'], 4, 0, 8, ['case', IF_STALE, 0.5, 1.5]],
  };

  let {
    traffic,
    altitudeUnit,
    showHitAreas,
  }: { traffic: TrafficStore; altitudeUnit: AltitudeUnit; showHitAreas: boolean } = $props();

  let source: MapLibreGeoJSONSource | undefined = $state();
  let updateQueue = Promise.resolve();

  $effect(() => {
    let activeSource = source;
    let activeAltitudeUnit = altitudeUnit;
    if (!activeSource) return;

    updateQueue = updateQueue.then(() =>
      activeSource.setData(trafficFeatureCollection(traffic.current.values(), activeAltitudeUnit)),
    );
  });

  onMount(() =>
    traffic.subscribe((update, currentTargets) => {
      let activeSource = source;
      if (!activeSource) return;

      updateQueue = updateQueue.then(() =>
        applyTrafficSourceUpdate(activeSource, update, currentTargets, altitudeUnit),
      );
    }),
  );
</script>

<GeoJSONSource
  id="traffic"
  maxzoom={24}
  promoteId="id"
  data={trafficFeatureCollection([], 'm')}
  bind:source
>
  <CircleLayer
    id="traffic-hit"
    paint={{ 'circle-radius': 24, 'circle-opacity': showHitAreas ? 0.2 : 0 }}
  />
  <SymbolLayer
    id="traffic-fixed"
    filter={[
      'any',
      ['==', ['get', 'trafficType'], 'balloon'],
      ['==', ['get', 'trackDegrees'], null],
    ]}
    layout={{
      ...TRAFFIC_LAYOUT,
      'icon-rotation-alignment': 'viewport',
    }}
    paint={TRAFFIC_PAINT}
  />

  <!--
  `icon-rotation-alignment` does not support data-driven styling, so we need to use a dedicated
  layer for non-rotating traffic.

  see https://github.com/maplibre/maplibre-gl-js/issues/3461
  -->

  <SymbolLayer
    id="traffic-directional"
    filter={[
      'all',
      ['!=', ['get', 'trafficType'], 'balloon'],
      ['!=', ['get', 'trackDegrees'], null],
    ]}
    layout={{
      ...TRAFFIC_LAYOUT,
      'icon-rotation-alignment': 'map',
      'icon-rotate': ['get', 'trackDegrees'],
    }}
    paint={TRAFFIC_PAINT}
  />
</GeoJSONSource>
