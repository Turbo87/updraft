<script lang="ts">
  import { getAppContext } from '$lib/app-context';
  import { languageOptions } from '$lib/language-options';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import SettingsIndexScreen from '$lib/SettingsIndexScreen.svelte';

  const { settings } = getAppContext();

  const activeLocale = $derived(settings.current.locale ?? getLocale());
  const language = $derived(languageOptions.find(({ locale }) => locale === activeLocale)?.label);
  const buildDate = $derived(
    new Intl.DateTimeFormat(activeLocale, { dateStyle: 'medium' }).format(
      new Date(__BUILD_TIMESTAMP__),
    ),
  );
</script>

<SettingsIndexScreen {language} {buildDate} />
