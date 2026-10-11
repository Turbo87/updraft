<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import MainMenuScreen from './MainMenuScreen.svelte';

  const { Story } = defineMeta({
    title: 'Screens/Main menu',
    component: MainMenuScreen,
    parameters: {
      layout: 'fullscreen',
      docs: {
        description: {
          component:
            'Use the Main Menu as the direct entry from the Flight View. One card groups the flight pages. One card opens Settings and About. Show the build date. Keep it visible as an em dash when its data is unavailable.',
        },
      },
    },
  });
</script>

<Story name="Current values" asChild>
  <div class="main-menu-story">
    <MainMenuScreen buildDate="15 Aug 2026" onQuit={() => {}} />
  </div>
</Story>
<Story name="Unknown values" asChild>
  <div class="main-menu-story">
    <MainMenuScreen />
  </div>
</Story>

<style>
  .main-menu-story {
    height: 100vh;
  }
</style>
