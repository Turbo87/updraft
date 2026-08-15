<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import LanguageSetting from './LanguageSetting.svelte';

  const { Story } = defineMeta({
    title: 'Components/LanguageSetting',
    component: LanguageSetting,
    parameters: {
      layout: 'centered',
      docs: {
        description: {
          component:
            'Use the language setting to select the application locale. Each language uses its own name so the options remain recognizable before the interface language changes. The native radio identifies the active locale. Selecting an option calls `onLocaleChange` with its locale code.',
        },
      },
    },
  });
</script>

<Story name="English" args={{ locale: 'en', onLocaleChange: () => {} }} />
<Story name="German" args={{ locale: 'de', onLocaleChange: () => {} }} />
