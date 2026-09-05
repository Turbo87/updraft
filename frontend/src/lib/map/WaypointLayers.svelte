<script lang="ts">
  import type {
    ExpressionSpecification,
    FilterSpecification,
    SymbolLayerSpecification,
  } from 'maplibre-gl';

  import { SymbolLayer } from 'svelte-maplibre-gl';

  import { waypointSymbols } from '$lib/waypoint-symbols';
  import { FONT_REGULAR } from './basemap-style';
  import { COLOR_SLATE_700, COLOR_VIOLET_700 } from './colors.generated';

  const WAYPOINT_KIND = {
    UNKNOWN: 0,
    AIRFIELD_GRASS: 2,
    OUTLANDING: 3,
    GLIDING_AIRFIELD: 4,
    AIRFIELD_SOLID: 5,
  } as const;
  const LANDABLE_KINDS = [
    WAYPOINT_KIND.AIRFIELD_GRASS,
    WAYPOINT_KIND.OUTLANDING,
    WAYPOINT_KIND.GLIDING_AIRFIELD,
    WAYPOINT_KIND.AIRFIELD_SOLID,
  ];
  const iconImage: ExpressionSpecification = [
    'match',
    ['get', 'kind'],
    WAYPOINT_KIND.UNKNOWN,
    'updraft-sdf:unknown',
    ...waypointSymbols.slice(1).flatMap(({ sprite }, kind) => [kind + 1, `updraft-sdf:${sprite}`]),
    'updraft-sdf:unknown',
  ];
  const visible: FilterSpecification = [
    'any',
    ['in', ['get', 'kind'], ['literal', LANDABLE_KINDS]],
    ['>=', ['zoom'], 6],
  ];
  const layout: NonNullable<SymbolLayerSpecification['layout']> = {
    'icon-image': iconImage,
    'icon-size': scaledWaypointSize(0.5),
    'icon-allow-overlap': true,
    'icon-ignore-placement': true,
  };
  const paint: NonNullable<SymbolLayerSpecification['paint']> = {
    'icon-color': ['match', ['get', 'kind'], LANDABLE_KINDS, COLOR_VIOLET_700, COLOR_SLATE_700],
    'icon-halo-color': '#ffffff',
    'icon-halo-width': scaledWaypointSize(1.5),
  };

  function scaledWaypointSize(size: number): ExpressionSpecification {
    return [
      'interpolate',
      ['linear'],
      ['zoom'],
      6,
      ['match', ['get', 'kind'], LANDABLE_KINDS, size * 0.5, size * 0.3],
      8,
      ['match', ['get', 'kind'], LANDABLE_KINDS, size, size * 0.8],
    ];
  }
</script>

<SymbolLayer id="waypoint-symbols" filter={visible} beforeId="traffic-fixed" {layout} {paint} />
<SymbolLayer
  id="waypoint-runways"
  beforeId="traffic-fixed"
  filter={['all', ['in', ['get', 'kind'], ['literal', LANDABLE_KINDS]], ['has', 'runwayDirection']]}
  layout={{
    'icon-image': 'updraft-sdf:runway',
    'icon-size': ['interpolate', ['linear'], ['zoom'], 6, 0.152, 8, 0.304],
    'icon-rotate': ['get', 'runwayDirection'],
    'icon-rotation-alignment': 'map',
    'icon-allow-overlap': true,
    'icon-ignore-placement': true,
  }}
  paint={{
    'icon-color': '#ffffff',
    'icon-halo-color': COLOR_VIOLET_700,
    'icon-halo-width': ['interpolate', ['linear'], ['zoom'], 6, 0.25, 8, 0.5],
  }}
/>
<SymbolLayer
  id="waypoint-labels"
  beforeId="traffic-fixed"
  minzoom={8}
  layout={{
    'text-field': ['get', 'name'],
    'text-font': FONT_REGULAR,
    'text-size': 12,
    'text-padding': 8,
    'text-variable-anchor': ['top', 'bottom'],
    'text-radial-offset': 0.5,
    'symbol-sort-key': ['match', ['get', 'kind'], LANDABLE_KINDS, 0, 1],
  }}
  paint={{ 'text-color': COLOR_SLATE_700, 'text-halo-color': '#ffffff', 'text-halo-width': 1.5 }}
/>
