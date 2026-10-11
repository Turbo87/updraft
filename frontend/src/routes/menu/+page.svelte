<script lang="ts">
  import { getAppContext } from '#lib/app-context.js';
  import ConfirmDialog from '#lib/ConfirmDialog.svelte';
  import MainMenuScreen from '#lib/MainMenuScreen.svelte';
  import { m } from '#lib/paraglide/messages.js';
  import { getLocale } from '#lib/paraglide/runtime.js';

  const { client, settings } = getAppContext();

  let quitDialogOpen = $state(false);

  const buildDate = $derived(
    new Intl.DateTimeFormat(settings.current.locale ?? getLocale(), { dateStyle: 'medium' }).format(
      new Date(__BUILD_TIMESTAMP__),
    ),
  );

  function quit(): void {
    void client.quit().catch((error: unknown) => {
      console.error('Failed to quit', error);
    });
  }
</script>

<MainMenuScreen {buildDate} onQuit={() => (quitDialogOpen = true)} />

<ConfirmDialog
  bind:open={quitDialogOpen}
  title={m.quit_app_confirm_title()}
  description={m.quit_app_hint()}
  cancelLabel={m.cancel()}
  confirmLabel={m.quit_app()}
  onCancel={() => (quitDialogOpen = false)}
  onConfirm={quit}
/>
