<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import SettingsIndexScreen from './SettingsIndexScreen.svelte';

  const { Story } = defineMeta({
    title: 'Screens/Settings index',
    component: SettingsIndexScreen,
    parameters: {
      layout: 'fullscreen',
      docs: {
        description: {
          component:
            'Use the Settings index as the direct entry from the Flight View. Each 56-pixel row opens one settings section. Show the current language and build date. Keep either value visible as an em dash when its data is unavailable.',
        },
      },
    },
  });
</script>

<Story name="Current values" asChild>
  <div class="settings-index-story">
    <SettingsIndexScreen buildDate="15 Aug 2026" language="English" />
  </div>
</Story>
<Story name="Unknown values" asChild>
  <div class="settings-index-story">
    <SettingsIndexScreen />
  </div>
</Story>

<style>
  .settings-index-story {
    height: 100vh;
  }
</style>
