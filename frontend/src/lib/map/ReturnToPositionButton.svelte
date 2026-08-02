<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';

  let { onClick }: { onClick: () => void } = $props();
</script>

<button type="button" aria-label={m.return_to_position()} onclick={onClick}>
  <span aria-hidden="true" class="i-mdi-crosshairs-gps icon"></span>
</button>

<style>
  button {
    position: absolute;
    inset-block-end: 1rem;
    inset-inline-end: 1rem;
    display: grid;
    place-items: center;
    width: 3rem;
    height: 3rem;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--color-overlay-surface);
    color: var(--color-overlay-text);
    cursor: pointer;
  }

  .icon {
    width: 1.5rem;
    height: 1.5rem;
  }
</style>
