<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import MapOverlayControl from './MapOverlayControl.svelte';

  const { Story } = defineMeta({
    title: 'Components/MapOverlayControl',
    component: MapOverlayControl,
    parameters: {
      docs: {
        description: {
          component:
            'Use a map overlay control for a familiar icon-only action in the Flight View. Every control is a round 56-pixel target with a required accessible label. Use a route link for a fixed destination and a callback button for an action. The translucent surface keeps the map visible. Pressing the control increases the scrim opacity without changing its color. The parent composition owns its position over the map.',
        },
      },
    },
  });
</script>

<Story name="Menu link" asChild>
  <div class="map-placeholder">
    <MapOverlayControl href="/settings" icon="i-mdi-menu" label="Settings" />
  </div>
</Story>

<Story name="Return to position" asChild>
  <div class="map-placeholder">
    <MapOverlayControl icon="i-mdi-crosshairs-gps" label="Return to position" onClick={() => {}} />
  </div>
</Story>

<style>
  .map-placeholder {
    display: grid;
    width: 100%;
    height: 8rem;
    place-items: center;
    background: light-dark(var(--color-slate-300), var(--color-slate-700));
  }
</style>
