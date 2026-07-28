<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import LanguageSetting from './LanguageSetting.svelte';

  const { Story } = defineMeta({
    title: 'Components/LanguageSetting',
    component: LanguageSetting,
  });
</script>

<Story name="English" args={{ locale: 'en', onLocaleChange: () => {} }} />
<Story name="German" args={{ locale: 'de', onLocaleChange: () => {} }} />
