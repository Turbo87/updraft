<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { fn } from 'storybook/test';

  import AirspaceSetting from './AirspaceSetting.svelte';

  const { Story } = defineMeta({
    title: 'Components/AirspaceSetting',
    component: AirspaceSetting,
    args: {
      onImport: fn(async () => ({ type: 'cancelled' as const })),
      onRemove: fn(async () => {}),
    },
    parameters: {
      layout: 'fullscreen',
      docs: {
        description: {
          component:
            'Each imported source has its own status and remove action. Import replaces the file with the same filename. Removal requires confirmation.',
        },
      },
    },
  });
</script>

<Story name="None" args={{ status: { generation: 0, sources: [] } }} />

<Story
  name="Active"
  args={{
    status: {
      generation: 1,
      sources: [{ type: 'active', sourceName: 'rheinland.txt', airspaceCount: 42 }],
    },
  }}
/>

<Story
  name="Unavailable"
  args={{
    status: {
      generation: 0,
      sources: [{ type: 'unavailable', sourceName: 'broken.txt', error: 'parseFailed' }],
    },
  }}
/>

<Story
  name="Multiple sources"
  args={{
    status: {
      generation: 3,
      sources: [
        { type: 'active', sourceName: 'germany.txt', airspaceCount: 1200 },
        { type: 'active', sourceName: 'france.txt', airspaceCount: 950 },
        { type: 'unavailable', sourceName: 'local.txt', error: 'parseFailed' },
      ],
    },
  }}
/>
