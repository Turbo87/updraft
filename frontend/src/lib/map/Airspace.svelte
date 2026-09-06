<script lang="ts">
  import type {
    ExpressionSpecification,
    GeoJSONSourceSpecification,
    LineLayerSpecification,
  } from 'maplibre-gl';

  import { FillLayer, GeoJSONSource, LineLayer } from 'svelte-maplibre-gl';

  import {
    COLOR_BLUE_600,
    COLOR_BLUE_700,
    COLOR_GREEN_700,
    COLOR_RED_600,
    COLOR_RED_700,
    COLOR_SLATE_600,
    COLOR_SLATE_700,
    COLOR_YELLOW_400,
    COLOR_YELLOW_600,
  } from './colors.generated';

  let {
    data,
    beforeId,
  }: {
    data: GeoJSONSourceSpecification['data'];
    beforeId: string;
  } = $props();

  const AIRSPACE_TYPE = {
    RESTRICTED: 1,
    DANGER: 2,
    PROHIBITED: 3,
    CONTROLLED_TOWER_REGION: 4,
    TRANSPONDER_MANDATORY_ZONE: 5,
    RADIO_MANDATORY_ZONE: 6,
    FLIGHT_INFORMATION_REGION: 10,
    UPPER_FLIGHT_INFORMATION_REGION: 11,
    AIRPORT_TRAFFIC_ZONE: 13,
    PROTECTED_AREA: 19,
    GLIDING_SECTOR: 21,
    TRAFFIC_INFORMATION_ZONE: 23,
    TRAFFIC_INFORMATION_AREA: 24,
    MILITARY_TRAINING_AREA: 25,
    LOW_ALTITUDE_OVERFLIGHT_RESTRICTION: 29,
    FIS_SECTOR: 33,
  } as const;

  const restrictedTypes = [
    AIRSPACE_TYPE.RESTRICTED,
    AIRSPACE_TYPE.DANGER,
    AIRSPACE_TYPE.PROHIBITED,
    AIRSPACE_TYPE.LOW_ALTITUDE_OVERFLIGHT_RESTRICTION,
  ];
  const informationTypes = [
    AIRSPACE_TYPE.FLIGHT_INFORMATION_REGION,
    AIRSPACE_TYPE.UPPER_FLIGHT_INFORMATION_REGION,
    AIRSPACE_TYPE.FIS_SECTOR,
  ];
  const blueZoneTypes = [
    AIRSPACE_TYPE.RADIO_MANDATORY_ZONE,
    AIRSPACE_TYPE.AIRPORT_TRAFFIC_ZONE,
    AIRSPACE_TYPE.TRAFFIC_INFORMATION_ZONE,
    AIRSPACE_TYPE.TRAFFIC_INFORMATION_AREA,
  ];
  const zoneBandTypes = [
    AIRSPACE_TYPE.CONTROLLED_TOWER_REGION,
    AIRSPACE_TYPE.GLIDING_SECTOR,
    ...blueZoneTypes,
    AIRSPACE_TYPE.MILITARY_TRAINING_AREA,
  ];
  const restrictedBandTypes = [...restrictedTypes, AIRSPACE_TYPE.PROTECTED_AREA];
  const dottedTypes = [AIRSPACE_TYPE.FIS_SECTOR, AIRSPACE_TYPE.MILITARY_TRAINING_AREA];
  const longDashTypes = [...restrictedTypes, AIRSPACE_TYPE.CONTROLLED_TOWER_REGION];
  const outlineColor: ExpressionSpecification = [
    'match',
    ['get', 'type'],
    restrictedTypes,
    COLOR_RED_700,
    AIRSPACE_TYPE.TRANSPONDER_MANDATORY_ZONE,
    COLOR_SLATE_700,
    AIRSPACE_TYPE.GLIDING_SECTOR,
    COLOR_YELLOW_600,
    AIRSPACE_TYPE.MILITARY_TRAINING_AREA,
    COLOR_SLATE_600,
    [AIRSPACE_TYPE.PROTECTED_AREA, ...informationTypes],
    COLOR_GREEN_700,
    COLOR_BLUE_700,
  ];
  const outlinePaint: NonNullable<LineLayerSpecification['paint']> = {
    'line-color': outlineColor,
    'line-width': ['match', ['get', 'type'], informationTypes, 1.5, 2],
    'line-opacity': ['match', ['get', 'type'], AIRSPACE_TYPE.GLIDING_SECTOR, 0.8, 1],
    'line-dasharray': [
      'match',
      ['get', 'type'],
      dottedTypes,
      ['literal', [0, 3]],
      AIRSPACE_TYPE.TRANSPONDER_MANDATORY_ZONE,
      ['literal', [4, 3, 1, 3]],
      blueZoneTypes,
      ['literal', [3, 3]],
      longDashTypes,
      ['literal', [4, 3]],
      ['literal', [1, 0]],
    ],
  };
  const bandPaint: NonNullable<LineLayerSpecification['paint']> = {
    'line-color': [
      'match',
      ['get', 'type'],
      AIRSPACE_TYPE.CONTROLLED_TOWER_REGION,
      COLOR_RED_600,
      AIRSPACE_TYPE.GLIDING_SECTOR,
      COLOR_YELLOW_400,
      blueZoneTypes,
      COLOR_BLUE_600,
      outlineColor,
    ],
    'line-opacity': ['match', ['get', 'type'], AIRSPACE_TYPE.GLIDING_SECTOR, 0.25, 0.2],
    'line-width': [
      'interpolate',
      ['linear'],
      ['zoom'],
      6,
      0,
      8,
      ['match', ['get', 'type'], AIRSPACE_TYPE.GLIDING_SECTOR, 10, 7],
    ],
    'line-offset': [
      'interpolate',
      ['linear'],
      ['zoom'],
      6,
      0,
      8,
      ['match', ['get', 'type'], AIRSPACE_TYPE.GLIDING_SECTOR, 5, 3.5],
    ],
  };
</script>

<GeoJSONSource id="airspace" maxzoom={24} promoteId="id" {data}>
  <FillLayer id="airspace-hit" {beforeId} paint={{ 'fill-opacity': 0 }} />
  <LineLayer
    id="airspace-inner-band"
    {beforeId}
    filter={[
      'any',
      ['in', ['get', 'type'], ['literal', [...zoneBandTypes, ...restrictedBandTypes]]],
      ['in', ['get', 'icaoClass'], ['literal', [0, 1, 2, 3]]],
    ]}
    layout={{
      'line-sort-key': ['match', ['get', 'type'], restrictedBandTypes, 2, zoneBandTypes, 0, 1],
    }}
    paint={bandPaint}
  />
  <LineLayer
    id="airspace-outline"
    {beforeId}
    layout={{
      'line-cap': ['match', ['get', 'type'], dottedTypes, 'round', 'butt'],
      'line-sort-key': [
        'match',
        ['get', 'type'],
        dottedTypes,
        4,
        AIRSPACE_TYPE.TRANSPONDER_MANDATORY_ZONE,
        3,
        blueZoneTypes,
        2,
        longDashTypes,
        1,
        0,
      ],
    }}
    paint={outlinePaint}
  />
</GeoJSONSource>
