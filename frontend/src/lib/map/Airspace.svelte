<script lang="ts">
  import type {
    ExpressionSpecification,
    FillLayerSpecification,
    GeoJSONSourceSpecification,
    LineLayerSpecification,
  } from 'maplibre-gl';

  import { FillLayer, GeoJSONSource, LineLayer } from 'svelte-maplibre-gl';

  import {
    COLOR_BLUE_600,
    COLOR_BLUE_700,
    COLOR_GREEN_600,
    COLOR_GREEN_700,
    COLOR_RED_600,
    COLOR_RED_700,
    COLOR_SLATE_500,
    COLOR_SLATE_600,
    COLOR_VIOLET_600,
    COLOR_VIOLET_700,
  } from '$lib/map/colors.generated';

  type FillPaint = NonNullable<FillLayerSpecification['paint']>;
  type LinePaint = NonNullable<LineLayerSpecification['paint']>;
  type Props = {
    data: GeoJSONSourceSpecification['data'];
    beforeId: string;
  };

  const AIRSPACE_TYPE = {
    RESTRICTED: 1,
    DANGER: 2,
    PROHIBITED: 3,
    CONTROLLED_TOWER_REGION: 4,
    TRANSPONDER_MANDATORY_ZONE: 5,
    RADIO_MANDATORY_ZONE: 6,
    TERMINAL_MANEUVERING_AREA: 7,
    AIRPORT_TRAFFIC_ZONE: 13,
    AIRWAY: 15,
    PROTECTED_AREA: 19,
    GLIDING_SECTOR: 21,
    TRANSPONDER_SETTING: 22,
    CONTROL_AREA: 26,
    ACC_SECTOR: 27,
    AERIAL_SPORTING_OR_RECREATIONAL_ACTIVITY: 28,
    LOW_ALTITUDE_OVERFLIGHT_RESTRICTION: 29,
    MILITARY_CONTROLLED_TOWER_REGION: 36,
  } as const;

  const AIRSPACE_CLASS = {
    A: 0,
    B: 1,
    C: 2,
    D: 3,
    E: 4,
  } as const;

  function airspaceStyleValue(
    controlled: string | number,
    prohibitedRestrictedDanger: string | number,
    mandatoryZone: string | number,
    glidingWave: string | number,
    other: string | number,
  ): ExpressionSpecification {
    return [
      'match',
      ['get', 'type'],
      [
        AIRSPACE_TYPE.RESTRICTED,
        AIRSPACE_TYPE.DANGER,
        AIRSPACE_TYPE.PROHIBITED,
        AIRSPACE_TYPE.PROTECTED_AREA,
        AIRSPACE_TYPE.LOW_ALTITUDE_OVERFLIGHT_RESTRICTION,
      ],
      prohibitedRestrictedDanger,
      [
        AIRSPACE_TYPE.TRANSPONDER_MANDATORY_ZONE,
        AIRSPACE_TYPE.RADIO_MANDATORY_ZONE,
        AIRSPACE_TYPE.TRANSPONDER_SETTING,
      ],
      mandatoryZone,
      [AIRSPACE_TYPE.GLIDING_SECTOR, AIRSPACE_TYPE.AERIAL_SPORTING_OR_RECREATIONAL_ACTIVITY],
      glidingWave,
      [
        AIRSPACE_TYPE.CONTROLLED_TOWER_REGION,
        AIRSPACE_TYPE.TERMINAL_MANEUVERING_AREA,
        AIRSPACE_TYPE.AIRPORT_TRAFFIC_ZONE,
        AIRSPACE_TYPE.AIRWAY,
        AIRSPACE_TYPE.CONTROL_AREA,
        AIRSPACE_TYPE.ACC_SECTOR,
        AIRSPACE_TYPE.MILITARY_CONTROLLED_TOWER_REGION,
      ],
      controlled,
      [
        'match',
        ['get', 'icaoClass'],
        [AIRSPACE_CLASS.A, AIRSPACE_CLASS.B, AIRSPACE_CLASS.C, AIRSPACE_CLASS.D, AIRSPACE_CLASS.E],
        controlled,
        other,
      ],
    ];
  }

  const AIRSPACE_FILL_PAINT: FillPaint = {
    'fill-color': airspaceStyleValue(
      COLOR_BLUE_600,
      COLOR_RED_600,
      COLOR_VIOLET_600,
      COLOR_GREEN_600,
      COLOR_SLATE_500,
    ),
    'fill-opacity': airspaceStyleValue(0.12, 0.18, 0.14, 0.12, 0.08),
  };

  const AIRSPACE_OUTLINE_PAINT: LinePaint = {
    'line-color': airspaceStyleValue(
      COLOR_BLUE_700,
      COLOR_RED_700,
      COLOR_VIOLET_700,
      COLOR_GREEN_700,
      COLOR_SLATE_600,
    ),
    'line-width': airspaceStyleValue(1.5, 2, 1.5, 1.5, 1),
  };

  let { data, beforeId }: Props = $props();
</script>

<GeoJSONSource id="airspace" maxzoom={24} {data}>
  <FillLayer id="airspace-hit" {beforeId} paint={{ 'fill-opacity': 0 }} />
  <FillLayer id="airspace-fill" {beforeId} paint={AIRSPACE_FILL_PAINT} />
  <LineLayer id="airspace-outline" {beforeId} paint={AIRSPACE_OUTLINE_PAINT} />
</GeoJSONSource>
