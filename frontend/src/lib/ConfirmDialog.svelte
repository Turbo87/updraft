<script lang="ts">
  import { AlertDialog } from 'bits-ui';

  import Button from './Button.svelte';

  type Props = {
    open: boolean;
    title: string;
    description: string;
    cancelLabel: string;
    confirmLabel: string;
    pending?: boolean;
    error?: string;
    onCancel: () => void;
    onConfirm: () => void;
  };

  let {
    open = $bindable(),
    title,
    description,
    cancelLabel,
    confirmLabel,
    pending = false,
    error,
    onCancel,
    onConfirm,
  }: Props = $props();

  let cancelButton = $state<HTMLButtonElement | null>(null);

  function focusCancel(event: Event) {
    event.preventDefault();
    cancelButton?.focus();
  }

  function handleEscape(event: KeyboardEvent) {
    if (pending) {
      event.preventDefault();
      return;
    }

    onCancel();
  }
</script>

<AlertDialog.Root bind:open>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="confirm-dialog-overlay" />
    <AlertDialog.Content
      class="confirm-dialog-content"
      onEscapeKeydown={handleEscape}
      onOpenAutoFocus={focusCancel}
    >
      <AlertDialog.Title class="confirm-dialog-title" level={2}>{title}</AlertDialog.Title>
      <AlertDialog.Description class="confirm-dialog-description">
        {description}
      </AlertDialog.Description>
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}
      <div class="actions">
        <AlertDialog.Cancel bind:ref={cancelButton} onclick={onCancel}>
          {#snippet child({ props })}
            <Button
              {...props}
              disabled={pending}
              size="large"
              variant="secondary"
              style="width: 100%"
            >
              {cancelLabel}
            </Button>
          {/snippet}
        </AlertDialog.Cancel>
        <AlertDialog.Action>
          {#snippet child({ props })}
            <Button
              {...props}
              loading={pending}
              size="large"
              variant="destructive"
              style="width: 100%"
              onclick={onConfirm}
            >
              {confirmLabel}
            </Button>
          {/snippet}
        </AlertDialog.Action>
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>

<style>
  :global(.confirm-dialog-overlay) {
    position: fixed;
    z-index: 100;
    inset: 0;
    background: var(--color-scrim);
  }

  :global(.confirm-dialog-content) {
    position: fixed;
    z-index: 101;
    top: 50%;
    left: 50%;
    display: grid;
    width: min(calc(100% - 2 * var(--space-5)), 28rem);
    padding: var(--space-6);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
    box-shadow: var(--shadow-modal);
    color: var(--color-text);
    translate: -50% -50%;
  }

  :global(.confirm-dialog-title) {
    margin: 0;
    font: var(--text-screen-title);
  }

  :global(.confirm-dialog-description) {
    margin: var(--space-3) 0 0;
    color: var(--color-text-muted);
  }

  .error {
    margin: var(--space-4) 0 0;
    color: var(--color-danger-subtle-text);
    font: var(--text-body);
    font-weight: 500;
  }

  .actions {
    display: grid;
    gap: var(--space-2);
    margin-block-start: var(--space-6);
  }

  @media (orientation: landscape) and (min-width: 36rem) {
    .actions {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
