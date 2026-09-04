<script lang="ts">
  import type {
    ExpressionSpecification,
    GeoJSONSourceSpecification,
    SymbolLayerSpecification,
  } from 'maplibre-gl';

  import { CircleLayer, GeoJSONSource, SymbolLayer } from 'svelte-maplibre-gl';

  import { COLOR_SLATE_700, COLOR_VIOLET_700 } from './colors.generated';

  let { data }: { data: GeoJSONSourceSpecification['data'] } = $props();
  const sprites = [
    'unknown',
    'waypoint-point',
    'waypoint-grass-airfield',
    'outlanding-bg',
    'waypoint-gliding-airfield',
    'waypoint-solid-airfield',
    'waypoint-mountain-pass',
    'waypoint-mountain-top',
    'waypoint-mast',
    'waypoint-vor',
    'waypoint-ndb',
    'waypoint-cooling-tower',
    'waypoint-dam',
    'waypoint-tunnel',
    'waypoint-bridge',
    'waypoint-power-plant',
    'waypoint-castle',
    'waypoint-intersection',
    'waypoint-marker',
    'waypoint-control-point',
    'waypoint-pg-takeoff',
    'waypoint-pg-landing',
  ];
  const iconImage: ExpressionSpecification = [
    'match',
    ['get', 'kind'],
    0,
    'updraft-sdf:unknown',
    ...sprites.slice(1).flatMap((sprite, kind) => [kind + 1, `updraft-sdf:${sprite}`]),
    'updraft-sdf:unknown',
  ];
  const layout: NonNullable<SymbolLayerSpecification['layout']> = {
    'icon-image': iconImage,
    'icon-size': 0.7,
    'icon-allow-overlap': true,
  };
  const paint: NonNullable<SymbolLayerSpecification['paint']> = {
    'icon-color': ['match', ['get', 'kind'], [2, 3, 4, 5], COLOR_VIOLET_700, COLOR_SLATE_700],
    'icon-halo-color': '#ffffff',
    'icon-halo-width': 1.5,
  };
</script>

<GeoJSONSource id="waypoints" {data}>
  <CircleLayer
    id="waypoint-hit"
    beforeId="traffic-fixed"
    paint={{ 'circle-radius': 12, 'circle-opacity': 0 }}
  />
  <SymbolLayer id="waypoint-symbols" beforeId="traffic-fixed" {layout} {paint} />
</GeoJSONSource>
