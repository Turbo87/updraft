<script lang="ts">
  import type { Locale } from '$lib/protocol/generated/Locale';

  import { resolve } from '$app/paths';

  import { getAppContext } from '$lib/app-context';
  import LanguageSetting from '$lib/LanguageSetting.svelte';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';

  const { client, settings } = getAppContext();
  const activeLocale = $derived(settings.current.locale ?? getLocale());

  function selectLocale(locale: Locale): void {
    void client.setLocale(locale).catch((error: unknown) => {
      console.error('Failed to set locale', error);
    });
  }
</script>

<main>
  <a class="back-link" href={resolve('/settings')}>{m.back_to_settings()}</a>
  <h1>{m.language_label()}</h1>
  <LanguageSetting locale={activeLocale} onLocaleChange={selectLocale} />
</main>

<style>
  main {
    min-height: 100%;
    padding: 1.5rem;
    background-color: var(--color-app-surface);
    color: var(--color-text);
  }

  .back-link {
    display: inline-block;
    margin-block-end: 1rem;
  }

  h1 {
    margin-block-start: 0;
  }
</style>
