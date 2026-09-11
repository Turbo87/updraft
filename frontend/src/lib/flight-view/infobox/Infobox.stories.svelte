<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Infobox from './Infobox.svelte';

  const { Story } = defineMeta({
    title: 'Flight View/Infobox',
    component: Infobox,
    args: { stale: false },
  });
</script>

<Story
  name="Altitude"
  args={{ label: 'Altitude', value: { kind: 'altitude', meters: 1245, unit: 'm' } }}
/>
<Story
  name="Stacked unit"
  args={{ label: 'TAS', value: { kind: 'speed', metersPerSecond: 30, unit: 'km/h' } }}
/>
<Story
  name="Climb"
  args={{ label: 'Vario', value: { kind: 'vertical-speed', metersPerSecond: 1.8, unit: 'm/s' } }}
/>
<Story
  name="Left bank"
  args={{ label: 'Bank angle', value: { kind: 'relative-angle', degrees: -12 } }}
/>
<Story
  name="Right bank"
  args={{ label: 'Bank angle', value: { kind: 'relative-angle', degrees: 12 } }}
/>
<Story name="Level" args={{ label: 'Bank angle', value: { kind: 'relative-angle', degrees: 0 } }} />
<Story
  name="Wind direction"
  args={{ label: 'Wind direction', value: { kind: 'direction', degrees: 248 } }}
/>
<Story
  name="Average vario"
  args={{
    label: 'Avg. vario',
    value: { kind: 'vertical-speed', metersPerSecond: 1.4, unit: 'm/s' },
  }}
/>
<Story
  name="Stale"
  args={{ label: 'TAS', value: { kind: 'speed', metersPerSecond: 30, unit: 'kt' }, stale: true }}
/>
<Story
  name="Unavailable"
  args={{ label: 'Altitude', value: { kind: 'altitude', meters: null, unit: 'm' } }}
/>
