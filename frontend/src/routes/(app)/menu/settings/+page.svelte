<script lang="ts">
  import { resolve } from '$app/paths';
  import { m } from '$lib/paraglide/messages.js';
  import DialogScreen from '$lib/dialog/DialogScreen.svelte';
</script>

<DialogScreen title={m.settings_title()} back>
  <nav class="list">
    <a href={resolve('/menu/settings/language')}>{m.language_label()}</a>
  </nav>
</DialogScreen>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
</style>
