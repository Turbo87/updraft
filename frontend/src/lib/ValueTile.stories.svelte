<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ValueTile from './ValueTile.svelte';

  const { Story } = defineMeta({
    title: 'Components/ValueTile',
    component: ValueTile,
    parameters: {
      layout: 'centered',
      docs: {
        description: {
          component:
            'Use a value tile for labelled flight data on an opaque screen or map overlay. The numeric value uses tabular Semi Condensed figures so updates do not shift its width. A slash stacks the unit as a fraction, while a degree sign aligns above the baseline. Stale values keep their last reading in the stale color. Show missing values as an em dash with the same stale treatment. Set `--value-tile-value-font` and the related unit-size properties when a composition needs a smaller tier. Set `--value-tile-surface` to the opaque map-overlay surface when the tile appears over the map.',
        },
      },
    },
  });
</script>

<Story name="Altitude" args={{ label: 'Altitude', value: '1245', unit: 'm' }} />
<Story name="Fractional unit" args={{ label: 'Vario', value: '+1.8', unit: 'm/s' }} />
<Story name="Degree unit" args={{ label: 'Track', value: '024', unit: '°' }} />
<Story name="Stale" args={{ label: 'Wind', stale: true, value: '248', unit: '°' }} />
<Story name="Unknown" args={{ label: 'Arrival', stale: true, value: '—' }} />

<Story name="Compact" asChild>
  <ValueTile
    --value-tile-degree-size="1rem"
    --value-tile-stacked-unit-size="0.6875rem"
    --value-tile-unit-size="0.875rem"
    --value-tile-value-font="var(--text-value-sm)"
    label="Elevation"
    value="412"
    unit="m"
  />
</Story>

<Story name="Map overlay" asChild>
  <div class="map-placeholder">
    <ValueTile
      --value-tile-surface="var(--color-map-overlay-surface)"
      label="Altitude"
      value="1245"
      unit="m"
    />
  </div>
</Story>

<style>
  .map-placeholder {
    width: 12rem;
    padding: var(--space-4);
    background: light-dark(var(--color-slate-300), var(--color-slate-700));
  }
</style>
