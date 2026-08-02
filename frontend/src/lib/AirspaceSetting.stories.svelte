<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import AirspaceSetting from './AirspaceSetting.svelte';

  const { Story } = defineMeta({
    title: 'Components/AirspaceSetting',
    component: AirspaceSetting,
    args: {
      onImport: async () => ({ type: 'cancelled' as const }),
      onRemove: async () => {},
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
