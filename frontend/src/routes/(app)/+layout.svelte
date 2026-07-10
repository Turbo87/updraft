<script lang="ts">
  import { resolve } from '$app/paths';
  import { m } from '$lib/paraglide/messages.js';
  import Map from '$lib/map/Map.svelte';
  import RouteDialog from '$lib/dialog/RouteDialog.svelte';

  let { children } = $props();
</script>

<!-- The map is the persistent app shell for this route group: it lives in the
     layout so it stays mounted (keeping its hot path alive) while dialog routes
     overlay it. Full-screen routes outside this group render without it. -->
<div class="shell">
  <Map />
  <a class="menu" href={resolve('/menu')} aria-label={m.menu_title()}>&#9776;</a>
</div>

<RouteDialog base="/">
  {@render children()}
</RouteDialog>

<style>
  .shell {
    position: fixed;
    inset: 0;
  }

  .menu {
    position: absolute;
    top: 0.5rem;
    left: 0.5rem;
  }
</style>
