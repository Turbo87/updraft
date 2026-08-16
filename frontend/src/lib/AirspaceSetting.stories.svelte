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
            'Use the airspace setting to manage one imported airspace file. The empty state offers an import action. The active state shows the source name and airspace count. An unavailable source stays visible with its load error so it can be replaced or removed. Mutations disable every action until they finish. Command failures appear as an alert. The component remains controlled through `status`, `onImport`, and `onRemove`.',
        },
      },
    },
  });
</script>

<Story name="None" args={{ status: { type: 'none' } }} />

<Story
  name="Active"
  args={{
    status: {
      type: 'active',
      sourceName: 'rheinland.txt',
      airspaceCount: 42,
      generation: 1,
    },
  }}
/>

<Story
  name="Unavailable"
  args={{
    status: {
      type: 'unavailable',
      sourceName: 'broken.txt',
      error: 'parseFailed',
    },
  }}
/>
