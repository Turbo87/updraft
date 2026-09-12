<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import TrafficSettings from './TrafficSettings.svelte';

  const { Story } = defineMeta({
    title: 'Screens/TrafficSettings',
    component: TrafficSettings,
    args: { enabled: true, setEnabled: async () => {} },
  });
</script>

<Story name="Correction enabled" />
<Story name="Correction disabled" args={{ enabled: false }} />
