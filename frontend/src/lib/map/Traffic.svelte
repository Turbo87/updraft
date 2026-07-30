<script lang="ts">
  import type {
    ExpressionSpecification,
    GeoJSONSource as MapLibreGeoJSONSource,
    SymbolLayerSpecification,
  } from 'maplibre-gl';
  import type { TrafficStore } from '$lib/stores/traffic.svelte';

  import { onMount } from 'svelte';
  import { GeoJSONSource, SymbolLayer } from 'svelte-maplibre-gl';

  import { applyTrafficSourceUpdate, trafficFeatureCollection } from './traffic';

  type SymbolLayout = NonNullable<SymbolLayerSpecification['layout']>;
  type SymbolPaint = NonNullable<SymbolLayerSpecification['paint']>;

  const TRAFFIC_OPACITY: ExpressionSpecification = [
    'case',
    ['boolean', ['get', 'stale'], false],
    0.45,
    1,
  ];

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
    'icon-size': 0.75,
    'icon-allow-overlap': true,
    'text-field': [
      'case',
      ['!=', ['get', 'altitudeMslMeters'], null],
      ['concat', ['to-string', ['round', ['number', ['get', 'altitudeMslMeters']]]], ' m'],
      '',
    ],
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
      '#f59e0b',
      'important',
      '#f97316',
      'urgent',
      '#dc2626',
      '#262626',
    ],
    'icon-opacity': TRAFFIC_OPACITY,
    'text-color': '#262626',
    'text-opacity': TRAFFIC_OPACITY,
  };

  let { traffic }: { traffic: TrafficStore } = $props();

  let source: MapLibreGeoJSONSource | undefined = $state();
  let updateQueue = Promise.resolve();

  function getSource() {
    return source;
  }

  function setSource(activeSource: MapLibreGeoJSONSource | undefined) {
    source = activeSource;
    if (!activeSource) return;

    updateQueue = updateQueue.then(() => {
      activeSource.setData(trafficFeatureCollection(traffic.current.values()));
    });
  }

  onMount(() =>
    traffic.subscribe((update, currentTargets) => {
      let activeSource = source;
      if (!activeSource) return;

      updateQueue = updateQueue.then(() =>
        applyTrafficSourceUpdate(activeSource, update, currentTargets),
      );
    }),
  );
</script>

<GeoJSONSource
  id="traffic"
  maxzoom={24}
  data={trafficFeatureCollection([])}
  bind:source={getSource, setSource}
>
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
