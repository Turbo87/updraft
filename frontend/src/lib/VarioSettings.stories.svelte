<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import VarioSettings from './VarioSettings.svelte';

  const { Story } = defineMeta({
    title: 'Screens/VarioSettings',
    component: VarioSettings,
    args: {
      method: 'smoothed20s',
      setMethod: async () => {},
      energyCompensation: true,
      setEnergyCompensation: async () => {},
    },
  });
</script>

<Story name="Smoothed twenty second average" />
<Story name="Normalized EMA" args={{ method: 'normalizedEma' }} />
<Story name="Twenty second average" args={{ method: 'average20s' }} />
<Story name="Thirty second average" args={{ method: 'average30s' }} />

<Story name="Compensation disabled" args={{ energyCompensation: false }} />
