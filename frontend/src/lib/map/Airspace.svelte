<script lang="ts">
  import type {
    ExpressionSpecification,
    FillLayerSpecification,
    GeoJSONSourceSpecification,
    LineLayerSpecification,
  } from 'maplibre-gl';

  import { FillLayer, GeoJSONSource, LineLayer } from 'svelte-maplibre-gl';

  type FillPaint = NonNullable<FillLayerSpecification['paint']>;
  type LinePaint = NonNullable<LineLayerSpecification['paint']>;
  type Props = {
    data: GeoJSONSourceSpecification['data'];
    beforeId: string;
  };

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
      ['P', 'R', 'Q', 'GP', 'OFR', 'TFR'],
      prohibitedRestrictedDanger,
      ['TMZ', 'RMZ'],
      mandatoryZone,
      ['GSEC', 'ASRA'],
      glidingWave,
      ['CTR', 'CTA', 'TMA', 'ATZ', 'AWY'],
      controlled,
      ['match', ['get', 'class'], ['A', 'B', 'C', 'D', 'E'], controlled, other],
    ];
  }

  const AIRSPACE_FILL_PAINT: FillPaint = {
    'fill-color': airspaceStyleValue('#2563eb', '#dc2626', '#9333ea', '#16a34a', '#64748b'),
    'fill-opacity': airspaceStyleValue(0.12, 0.18, 0.14, 0.12, 0.08),
  };

  const AIRSPACE_OUTLINE_PAINT: LinePaint = {
    'line-color': airspaceStyleValue('#1d4ed8', '#b91c1c', '#7e22ce', '#15803d', '#475569'),
    'line-width': airspaceStyleValue(1.5, 2, 1.5, 1.5, 1),
  };

  let { data, beforeId }: Props = $props();
</script>

<GeoJSONSource id="airspace" maxzoom={24} {data}>
  <FillLayer id="airspace-fill" {beforeId} paint={AIRSPACE_FILL_PAINT} />
  <LineLayer id="airspace-outline" {beforeId} paint={AIRSPACE_OUTLINE_PAINT} />
</GeoJSONSource>
