<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { fn } from 'storybook/test';

  import WaypointSettings from './WaypointSettings.svelte';

  const { Story } = defineMeta({
    title: 'Components/WaypointSettings',
    component: WaypointSettings,
    args: { onImport: fn(async () => ({ type: 'cancelled' as const })) },
    parameters: { layout: 'fullscreen' },
  });
</script>

<Story name="Empty" args={{ status: { generation: 0, sources: [] } }} />
<Story
  name="Multiple files and warnings"
  args={{
    status: {
      generation: 1,
      sources: [
        {
          type: 'active',
          sourceName: 'local.cup',
          waypointCount: 42,
          warnings: [{ line: 4, message: 'Skipped waypoint: invalid latitude' }],
        },
        { type: 'unavailable', sourceName: 'regional.cup', error: 'readFailed' },
      ],
    },
  }}
/>
