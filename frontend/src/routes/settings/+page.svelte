<script lang="ts">
  import { getAppContext } from '$lib/app-context';
  import { languageOptions } from '$lib/language-options';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import SettingsIndexScreen from '$lib/SettingsIndexScreen.svelte';

  const { client, settings } = getAppContext();

  const activeLocale = $derived(settings.current.locale ?? getLocale());
  const language = $derived(languageOptions.find(({ locale }) => locale === activeLocale)?.label);
  const buildDate = $derived(
    new Intl.DateTimeFormat(activeLocale, { dateStyle: 'medium' }).format(
      new Date(__BUILD_TIMESTAMP__),
    ),
  );

  function quit(): void {
    void client.quit().catch((error: unknown) => {
      console.error('Failed to quit', error);
    });
  }
</script>

<SettingsIndexScreen {language} {buildDate} onQuit={quit} />
