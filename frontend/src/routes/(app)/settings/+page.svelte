<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import Row from '$lib/dialog/Row.svelte';
</script>

<ul class="list">
  <li><Row href="/settings/map">{m.map_title()}</Row></li>
  <li><Row href="/settings/units">{m.units_title()}</Row></li>
  <li><Row href="/settings/about">{m.about_title()}</Row></li>
</ul>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
