<script lang="ts">
  import type {
    ExpressionSpecification,
    FilterSpecification,
    GeoJSONSourceSpecification,
    SymbolLayerSpecification,
  } from 'maplibre-gl';

  import { CircleLayer, GeoJSONSource, SymbolLayer } from 'svelte-maplibre-gl';

  import { FONT_REGULAR } from './basemap-style';
  import { COLOR_SLATE_700, COLOR_VIOLET_700 } from './colors.generated';

  let { data }: { data: GeoJSONSourceSpecification['data'] } = $props();
  const sprites = [
    'unknown',
    'waypoint-point',
    'waypoint-airfield',
    'outlanding-bg',
    'waypoint-airfield',
    'waypoint-airfield',
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
  const visible: FilterSpecification = [
    'any',
    ['in', ['get', 'kind'], ['literal', [2, 3, 4, 5]]],
    ['>=', ['zoom'], 6],
  ];
  const layout: NonNullable<SymbolLayerSpecification['layout']> = {
    'icon-image': iconImage,
    'icon-size': ['interpolate', ['linear'], ['zoom'], 6, 0.28, 8, 0.56],
    'icon-allow-overlap': true,
    'icon-ignore-placement': true,
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
  <SymbolLayer id="waypoint-symbols" filter={visible} beforeId="traffic-fixed" {layout} {paint} />
  <SymbolLayer
    id="waypoint-runways"
    beforeId="traffic-fixed"
    filter={['all', ['in', ['get', 'kind'], ['literal', [2, 3, 4, 5]]], ['has', 'runwayDirection']]}
    layout={{
      'icon-image': 'updraft-sdf:runway',
      'icon-size': ['interpolate', ['linear'], ['zoom'], 6, 0.152, 8, 0.304],
      'icon-rotate': ['get', 'runwayDirection'],
      'icon-rotation-alignment': 'map',
      'icon-allow-overlap': true,
      'icon-ignore-placement': true,
    }}
    paint={{ 'icon-color': '#ffffff', 'icon-halo-color': COLOR_VIOLET_700, 'icon-halo-width': 0.5 }}
  />
  <SymbolLayer
    id="waypoint-labels"
    beforeId="traffic-fixed"
    minzoom={8}
    layout={{
      'text-field': ['get', 'name'],
      'text-font': FONT_REGULAR,
      'text-size': 13,
      'text-variable-anchor': ['top', 'bottom'],
      'text-radial-offset': 1.2,
      'symbol-sort-key': ['match', ['get', 'kind'], [2, 3, 4, 5], 0, 1],
    }}
    paint={{ 'text-color': COLOR_SLATE_700, 'text-halo-color': '#ffffff', 'text-halo-width': 1.5 }}
  />
</GeoJSONSource>
