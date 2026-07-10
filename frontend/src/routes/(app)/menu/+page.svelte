<script lang="ts">
  import { resolve } from '$app/paths';
  import { m } from '$lib/paraglide/messages.js';
  import DialogScreen from '$lib/dialog/DialogScreen.svelte';
</script>

<DialogScreen title={m.menu_title()}>
  <nav class="list">
    <a href={resolve('/menu/settings')}>{m.settings_title()}</a>
  </nav>
</DialogScreen>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
</style>
