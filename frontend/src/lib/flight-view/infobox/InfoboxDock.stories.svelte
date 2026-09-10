<script module lang="ts">
  import type { ComponentProps } from 'svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import { EMPTY_INSTRUMENTS } from '$lib/stores/instruments.svelte';
  import { flightInfoboxes } from './fields';
  import InfoboxDock from './InfoboxDock.svelte';
  import { INFOBOX_INSTRUMENTS } from './instruments.fixture';

  const units = { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' } as const;
  const { Story } = defineMeta({
    title: 'Flight View/InfoboxDock',
    component: InfoboxDock,
    parameters: { layout: 'fullscreen' },
  });
</script>

{#snippet template(args: ComponentProps<typeof InfoboxDock>)}
  <div class="preview">
    <div class="map-placeholder"></div>
    <InfoboxDock {...args} />
  </div>
{/snippet}

<Story
  name="Current and stale"
  args={{ infoboxes: flightInfoboxes(INFOBOX_INSTRUMENTS, units, 11.25) }}
  {template}
/>
<Story
  name="Unavailable"
  args={{ infoboxes: flightInfoboxes(EMPTY_INSTRUMENTS, units, 11) }}
  {template}
/>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .map-placeholder {
    flex: 1;
    background: var(--color-screen-surface-sunken);
  }
  @media (orientation: landscape) {
    .preview {
      flex-direction: row;
    }
  }
</style>
