<script lang="ts">
  import MapOverlayControl from '$lib/MapOverlayControl.svelte';
  import { m } from '$lib/paraglide/messages.js';

  type Props = { onClick: () => void };

  let { onClick }: Props = $props();
</script>

<div class="return-to-position">
  <MapOverlayControl icon="i-mdi-crosshairs-gps" label={m.return_to_position()} {onClick} />
</div>

<style>
  .return-to-position {
    position: absolute;
    right: calc(1rem + var(--safe-area-right));
    bottom: calc(1rem + var(--safe-area-bottom));
  }
</style>
