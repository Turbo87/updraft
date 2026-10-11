<script lang="ts">
  import { getAppContext } from '#lib/app-context.js';
  import { languageOptions } from '#lib/language-options.js';
  import { getLocale } from '#lib/paraglide/runtime.js';
  import SettingsIndexScreen from '#lib/SettingsIndexScreen.svelte';

  const { settings, enrouteCatalog } = getAppContext();

  const activeLocale = $derived(settings.current.locale ?? getLocale());
  const language = $derived(languageOptions.find(({ locale }) => locale === activeLocale)?.label);
</script>

<SettingsIndexScreen {language} updateCount={enrouteCatalog.updates?.length ?? 0} />
