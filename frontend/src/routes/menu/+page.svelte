<script lang="ts">
  import { getAppContext } from '#lib/app-context.js';
  import ConfirmDialog from '#lib/ConfirmDialog.svelte';
  import MainMenuScreen from '#lib/MainMenuScreen.svelte';
  import { m } from '#lib/paraglide/messages.js';
  import { getLocale } from '#lib/paraglide/runtime.js';

  const { client, settings, trail } = getAppContext();

  let newFlightDialogOpen = $state(false);
  let newFlightPending = $state(false);
  let newFlightFailed = $state(false);
  let quitDialogOpen = $state(false);

  const buildDate = $derived(
    new Intl.DateTimeFormat(settings.current.locale ?? getLocale(), { dateStyle: 'medium' }).format(
      new Date(__BUILD_TIMESTAMP__),
    ),
  );

  function openNewFlightDialog(): void {
    newFlightFailed = false;
    newFlightDialogOpen = true;
  }

  async function startNewFlight(): Promise<void> {
    newFlightPending = true;
    newFlightFailed = false;
    try {
      await client.startNewFlight();
      newFlightDialogOpen = false;
    } catch {
      newFlightFailed = true;
    } finally {
      newFlightPending = false;
    }
  }

  function quit(): void {
    void client.quit().catch((error: unknown) => {
      console.error('Failed to quit', error);
    });
  }
</script>

<MainMenuScreen
  {buildDate}
  onNewFlight={trail.hasRecording ? openNewFlightDialog : undefined}
  onQuit={() => (quitDialogOpen = true)}
/>

<ConfirmDialog
  bind:open={newFlightDialogOpen}
  title={m.new_flight_confirm_title()}
  description={m.new_flight_confirm_hint()}
  cancelLabel={m.cancel()}
  confirmLabel={m.new_flight()}
  pending={newFlightPending}
  error={newFlightFailed ? m.new_flight_failed() : undefined}
  onCancel={() => (newFlightDialogOpen = false)}
  onConfirm={() => void startNewFlight()}
/>

<ConfirmDialog
  bind:open={quitDialogOpen}
  title={m.quit_app_confirm_title()}
  description={m.quit_app_hint()}
  cancelLabel={m.cancel()}
  confirmLabel={m.quit_app()}
  onCancel={() => (quitDialogOpen = false)}
  onConfirm={quit}
/>
