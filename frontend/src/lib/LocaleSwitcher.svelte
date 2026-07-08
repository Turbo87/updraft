<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale, locales } from '$lib/paraglide/runtime.js';
  import { setLocale } from '$lib/i18n.svelte.js';
</script>

<nav>
  {m.language_label()}:
  {#each locales as locale (locale)}
    <button type="button" disabled={locale === getLocale()} onclick={() => setLocale(locale)}>
      {locale.toUpperCase()}
    </button>
  {/each}
</nav>
