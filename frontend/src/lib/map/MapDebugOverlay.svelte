<script lang="ts">
  import type { Map } from 'maplibre-gl';

  let { map }: { map: Map | undefined } = $props();

  let visible = $state(false);
  let showTileBoundaries = $state(false);

  let zoom = $state(0);
  let lng = $state(0);
  let lat = $state(0);

  function syncView() {
    if (!map) return;
    zoom = map.getZoom();
    const center = map.getCenter();
    lng = center.lng;
    lat = center.lat;
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== 'd' && event.key !== 'D') return;

    const target = event.target as HTMLElement | null;
    if (
      target &&
      (target.isContentEditable ||
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.tagName === 'SELECT')
    ) {
      return;
    }

    visible = !visible;
  }

  // Keep the readout in sync with the map while the overlay is visible.
  $effect(() => {
    if (!map || !visible) return;
    syncView();
    map.on('move', syncView);
    return () => map.off('move', syncView);
  });

  // MapLibre's tile-boundary debug mode outlines each tile and labels it with
  // its tile ID, which is handy when debugging map rendering.
  $effect(() => {
    if (map) map.showTileBoundaries = showTileBoundaries;
  });
</script>

<svelte:window onkeydown={onKeydown} />

{#if visible}
  <div class="map-debug-overlay">
    <dl>
      <dt>Zoom</dt>
      <dd>{zoom.toFixed(2)}</dd>
      <dt>Center</dt>
      <dd>{lat.toFixed(5)}, {lng.toFixed(5)}</dd>
    </dl>
    <label>
      <input type="checkbox" bind:checked={showTileBoundaries} />
      Tile boundaries
    </label>
  </div>
{/if}

<style>
  .map-debug-overlay {
    position: absolute;
    top: 0.5rem;
    left: 0.5rem;
    z-index: 10;
    padding: 0.5rem 0.75rem;
    border-radius: 0.25rem;
    background: rgba(0, 0, 0, 0.75);
    color: #fff;
    font-family: monospace;
    font-size: 0.75rem;
    line-height: 1.4;
    pointer-events: auto;
  }

  dl {
    display: grid;
    grid-template-columns: auto auto;
    gap: 0 0.5rem;
    margin: 0 0 0.5rem;
  }

  dt {
    font-weight: bold;
  }

  dd {
    margin: 0;
  }

  label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    cursor: pointer;
  }
</style>
