<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import DialogScreen from '$lib/dialog/DialogScreen.svelte';
  import LocaleSwitcher from '$lib/LocaleSwitcher.svelte';
</script>

<DialogScreen title={m.language_label()} back>
  <LocaleSwitcher />
</DialogScreen>
