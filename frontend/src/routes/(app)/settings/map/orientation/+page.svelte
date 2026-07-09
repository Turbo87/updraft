<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import { settings } from '$lib/settings.svelte';
</script>

<label class="row">
  <input type="radio" name="orientation" value="north-up" bind:group={settings.orientation} />
  {m.orientation_north_up()}
</label>
<label class="row">
  <input type="radio" name="orientation" value="track-up" bind:group={settings.orientation} />
  {m.orientation_track_up()}
</label>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 3rem;
  }
</style>
