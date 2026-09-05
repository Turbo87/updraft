<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import GlidePerformanceControls from './GlidePerformanceControls.svelte';

  const { Story } = defineMeta({
    title: 'Settings/Flight controls',
    component: GlidePerformanceControls,
    args: {
      macCready: 0,
      bugs: 0,
      ballast: 0,
      unit: 'm/s',
      setMacCready: async () => {},
      setBugs: async () => {},
      setBallast: async () => {},
    },
  });
</script>

<Story name="Default" />
<Story name="Bugs" args={{ bugs: 10 }} />
<Story name="Ballast" args={{ ballast: 100 }} />
<Story name="Knots" args={{ macCready: 1.5, unit: 'kt' }} />
<Story name="Feet per minute" args={{ macCready: 1.5, unit: 'ft/min' }} />
