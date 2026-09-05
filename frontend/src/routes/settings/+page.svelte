<script lang="ts">
  import { getAppContext } from '$lib/app-context';
  import ConfirmDialog from '$lib/ConfirmDialog.svelte';
  import { languageOptions } from '$lib/language-options';
  import { m } from '$lib/paraglide/messages.js';
  import { getLocale } from '$lib/paraglide/runtime.js';
  import SettingsIndexScreen from '$lib/SettingsIndexScreen.svelte';

  const { client, settings } = getAppContext();

  let quitDialogOpen = $state(false);

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

<SettingsIndexScreen {language} {buildDate} onQuit={() => (quitDialogOpen = true)} />

<ConfirmDialog
  bind:open={quitDialogOpen}
  title={m.quit_app_confirm_title()}
  description={m.quit_app_hint()}
  cancelLabel={m.cancel()}
  confirmLabel={m.quit_app()}
  onCancel={() => (quitDialogOpen = false)}
  onConfirm={quit}
/>
