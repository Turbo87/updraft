<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import Row from '$lib/dialog/Row.svelte';
  import { settings } from '$lib/settings.svelte';
</script>

<label class="toggle">
  <input type="checkbox" bind:checked={settings.airspaceLabels} />
  {m.settings_airspace_labels()}
</label>

<ul class="list">
  <li>
    <Row
      href="/settings/map/orientation"
      value={settings.orientation === 'north-up'
        ? m.orientation_north_up()
        : m.orientation_track_up()}
    >
      {m.orientation_title()}
    </Row>
  </li>
</ul>

<style>
  .toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 3rem;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
