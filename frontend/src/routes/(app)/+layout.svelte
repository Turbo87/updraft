<script lang="ts">
  import { page } from '$app/state';
  import { resolve } from '$app/paths';
  import { m } from '$lib/paraglide/messages.js';
  import Map from '$lib/map/Map.svelte';
  import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
  import RouteDialog from '$lib/dialog/RouteDialog.svelte';

  let { children } = $props();

  // Each dialog screen's `load` returns a `title` getter (a Paraglide message, so
  // it re-resolves when the locale changes) and a `back` target. On an error, the
  // load failed so fall back to a generic error title.
  let title = $derived(page.error ? m.error_title() : (page.data.title?.() ?? ''));
  let back = $derived(page.data.back ?? null);
</script>

<!-- The map is the persistent app shell for this route group: it lives in the
     layout so it stays mounted (keeping its hot path alive) while dialog routes
     overlay it. Full-screen routes outside this group render without it. -->
<div class="shell">
  <Map />
  <div class="overlay">
    <a class="control" href={resolve('/settings')} aria-label={m.settings_title()}>&#9881;</a>
    <LocaleSwitcher />
  </div>
</div>

<RouteDialog base="/" {title} {back} backLabel={m.dialog_back()} closeLabel={m.dialog_close()}>
  {@render children()}
</RouteDialog>

<style>
  .shell {
    position: fixed;
    inset: 0;
  }

  .overlay {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .control {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2.75rem;
    height: 2.75rem;
    border-radius: 0.5rem;
    background: rgba(255, 255, 255, 0.9);
    color: #1a1a1a;
    font-size: 1.3rem;
    line-height: 1;
    text-decoration: none;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
  }
</style>
