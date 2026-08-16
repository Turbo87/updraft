<script lang="ts">
  import type { Locale } from '$lib/protocol/generated/Locale';

  import { getAppContext } from '$lib/app-context';
  import LanguageSetting from '$lib/LanguageSetting.svelte';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import ScreenScaffold from '$lib/ScreenScaffold.svelte';

  const { client, settings } = getAppContext();
  const activeLocale = $derived(settings.current.locale ?? getLocale());

  function selectLocale(locale: Locale): void {
    void client.setLocale(locale).catch((error: unknown) => {
      console.error('Failed to set locale', error);
    });
  }
</script>

<ScreenScaffold backHref="/settings" backLabel={m.back_to_settings()} title={m.language_label()}>
  <LanguageSetting locale={activeLocale} onLocaleChange={selectLocale} />
</ScreenScaffold>
