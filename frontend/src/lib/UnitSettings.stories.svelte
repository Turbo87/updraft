<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import UnitSettings from './UnitSettings.svelte';

  const { Story } = defineMeta({
    title: 'Components/UnitSettings',
    component: UnitSettings,
    parameters: {
      layout: 'centered',
      docs: {
        description: {
          component:
            'Use unit settings to choose altitude, distance, speed, and vertical-speed units. Each inline group keeps its complete option set visible and reports a complete `UnitSettings` value when the selection changes. The component remains controlled through `units` and `onUnitsChange`.',
        },
      },
    },
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
