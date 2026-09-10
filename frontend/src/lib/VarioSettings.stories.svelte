<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import VarioSettings from './VarioSettings.svelte';

  const { Story } = defineMeta({
    title: 'Screens/VarioSettings',
    component: VarioSettings,
    args: { method: 'normalizedEma', setMethod: async () => {} },
  });
</script>

<Story name="Normalized EMA" />
<Story name="Twenty second average" args={{ method: 'average20s' }} />
<Story name="Thirty second average" args={{ method: 'average30s' }} />
