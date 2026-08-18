<script module lang="ts">
  import type { ComponentProps } from 'svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import AboutScreen from './AboutScreen.svelte';

  const BUILD_TIMESTAMP = '2026-08-12T07:14:22.000Z';
  const FULL_COMMIT_SHA = 'a1c93f456789abcdef0123456789abcdef012345';
  const attributions = [
    'Base map tiles by <a href="https://openfreemap.org">OpenFreeMap</a>, data © <a href="https://www.openstreetmap.org/copyright">OpenStreetMap contributors</a>.',
  ];

  type Args = ComponentProps<typeof AboutScreen>;

  const { Story } = defineMeta({
    title: 'Screens/About',
    component: AboutScreen,
    args: {
      attributions,
      commitSha: FULL_COMMIT_SHA,
      locale: 'en',
      timestamp: BUILD_TIMESTAMP,
    },
    parameters: {
      layout: 'fullscreen',
      docs: {
        description: {
          component:
            'The About screen identifies Updraft and records the exact application build. It links to the source repository and lists each map attribution as safe text with separate 48-pixel link rows. An unknown build keeps the commit row visible. The data-credits section is absent when the map has no source attribution. Licence information remains readable prose.',
        },
      },
    },
  });
</script>

{#snippet template(args: Args)}
  <div class="about-screen-story">
    <AboutScreen {...args} />
  </div>
{/snippet}

<Story name="Complete build" {template} />

<Story name="Unknown build" args={{ attributions: [], commitSha: undefined }} {template} />

<style>
  .about-screen-story {
    height: 100vh;
  }
</style>
