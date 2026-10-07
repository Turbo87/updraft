<script lang="ts">
  import type { Locale } from '#lib/protocol/generated/Locale.js';

  import { resolve } from '$app/paths';

  import { getAppContext } from '#lib/app-context.js';
  import LanguageSetting from '#lib/LanguageSetting.svelte';
  import { m } from '#lib/paraglide/messages.js';
  import { getLocale } from '#lib/paraglide/runtime.js';
  import ScreenScaffold from '#lib/ScreenScaffold.svelte';

  const { client, settings } = getAppContext();
  const activeLocale = $derived(settings.current.locale ?? getLocale());

  function selectLocale(locale: Locale): void {
    void client.changeSetting({ type: 'locale', locale }).catch((error: unknown) => {
      console.error('Failed to set locale', error);
    });
  }
</script>

<ScreenScaffold
  backHref={resolve('/settings')}
  backLabel={m.back_to_settings()}
  title={m.language_label()}
>
  <LanguageSetting locale={activeLocale} onLocaleChange={selectLocale} />
</ScreenScaffold>
