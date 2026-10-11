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
            'Use the Settings index for saved settings. The Main Menu opens it. Three cards group flight settings, data and devices, and application preferences. Each 56-pixel row opens one destination. Show the current language. Keep it visible as an em dash when its data is unavailable.',
        },
      },
    },
  });
</script>

<Story name="Current values" asChild>
  <div class="settings-index-story">
    <SettingsIndexScreen language="English" />
  </div>
</Story>
<Story name="Unknown values" asChild>
  <div class="settings-index-story">
    <SettingsIndexScreen />
  </div>
</Story>
<Story name="Updates available" asChild>
  <div class="settings-index-story">
    <SettingsIndexScreen language="English" updateCount={2} />
  </div>
</Story>

<style>
  .settings-index-story {
    height: 100vh;
  }
</style>
