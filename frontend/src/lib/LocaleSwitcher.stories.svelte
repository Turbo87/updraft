<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import { setLocale } from '$lib/i18n.svelte.js';
  import LocaleSwitcher from './LocaleSwitcher.svelte';

  const { Story } = defineMeta({
    title: 'Components/LocaleSwitcher',
    component: LocaleSwitcher,
    beforeEach() {
      let previousLocale = getLocale();
      setLocale('en');
      return () => setLocale(previousLocale);
    },
  });
</script>

<Story name="Default" />
