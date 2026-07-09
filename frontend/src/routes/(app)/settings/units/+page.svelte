<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import { settings } from '$lib/settings.svelte';
</script>

<label class="row">
  <input type="radio" name="units" value="metric" bind:group={settings.units} />
  {m.units_metric()}
</label>
<label class="row">
  <input type="radio" name="units" value="imperial" bind:group={settings.units} />
  {m.units_imperial()}
</label>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 3rem;
  }
</style>
