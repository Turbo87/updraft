<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { fn } from 'storybook/test';

  import DataLibrary from './DataLibrary.svelte';

  const { Story } = defineMeta({
    title: 'Screens/DataLibrary',
    component: DataLibrary,
    args: {
      onRemove: fn(),
      airspace: { generation: 0, sources: [] },
      waypoints: { generation: 0, sources: [] },
    },
    parameters: { layout: 'fullscreen' },
  });
</script>

<Story name="Empty" />
<Story
  name="Imported files"
  args={{
    airspace: {
      generation: 1,
      sources: [
        { type: 'active', sourceName: 'Germany.txt', airspaceCount: 1200 },
        { type: 'unavailable', sourceName: 'alps_gliding_sectors.txt', error: 'parseFailed' },
        { type: 'disabled', sourceName: 'Local airspace.txt' },
      ],
    },
    waypoints: {
      generation: 1,
      sources: [
        {
          type: 'active',
          sourceName: 'club_outlandings.cup',
          waypointCount: 86,
          warnings: [
            { line: 4, message: 'Skipped waypoint' },
            { line: 5, message: 'Ignored field' },
          ],
        },
        {
          type: 'disabled',
          sourceName:
            'A long imported filename that must wrap without hiding its disabled state.cup',
        },
        { type: 'unavailable', sourceName: 'regional.cup', error: 'readFailed' },
      ],
    },
  }}
/>
