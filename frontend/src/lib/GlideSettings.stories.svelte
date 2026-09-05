<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import GlideSettings from './GlideSettings.svelte';

  const { Story } = defineMeta({
    title: 'Screens/GlideSettings',
    component: GlideSettings,
    args: {
      polar: 'LS 8',
      getPolars: async () => ['LS 8', 'LS 8-18'],
      setPolar: async () => {},
    },
  });
</script>

<Story name="Default" />
<Story name="Selected polar" args={{ polar: 'LS 8-18' }} />
<Story
  name="Catalog unavailable"
  args={{
    getPolars: async () => {
      throw new Error('Driver stopped');
    },
  }}
/>
