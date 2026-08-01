<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import UnitSettings from './UnitSettings.svelte';

  const { Story } = defineMeta({
    title: 'Components/UnitSettings',
    component: UnitSettings,
  });
</script>

<Story
  name="Metric"
  args={{
    units: {
      altitude: 'm',
      distance: 'km',
      speed: 'km/h',
      verticalSpeed: 'm/s',
    },
    onUnitsChange: () => {},
  }}
/>

<Story
  name="Aviation"
  args={{
    units: {
      altitude: 'ft',
      distance: 'nm',
      speed: 'kt',
      verticalSpeed: 'ft/min',
    },
    onUnitsChange: () => {},
  }}
/>
