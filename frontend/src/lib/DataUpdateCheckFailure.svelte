<script lang="ts">
  import Button from './Button.svelte';
  import { m } from './paraglide/messages.js';
  import { getLocale } from './paraglide/runtime.js';
  import ResponsiveCard from './ResponsiveCard.svelte';

  type Props = {
    checkedAt?: number;
    refreshing?: boolean;
    onRetry: () => Promise<void>;
  };
  let { checkedAt, refreshing = false, onRetry }: Props = $props();
  let pending = $state(false);
  let error = $state(false);

  async function retry() {
    pending = true;
    error = false;
    try {
      await onRetry();
    } catch {
      error = true;
    } finally {
      pending = false;
    }
  }
</script>

<ResponsiveCard>
  <div class="notice">
    <span aria-hidden="true" class="i-mdi-alert-circle-outline"></span>
    <div role="status">
      <strong>{m.data_update_check_unavailable()}</strong>
      <p>{m.data_basemap()} · Enroute</p>
      <p>
        {checkedAt === undefined
          ? m.data_never_checked()
          : m.data_catalog_last_checked({
              date: new Intl.DateTimeFormat(getLocale(), {
                dateStyle: 'medium',
                timeStyle: 'short',
              }).format(checkedAt),
            })}
      </p>
      {#if error}<p role="alert">{m.data_catalog_refresh_failed()}</p>{/if}
    </div>
    <Button variant="secondary" disabled={pending || refreshing} onclick={retry}>{m.retry()}</Button
    >
  </div>
</ResponsiveCard>

<style>
  .notice {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding-block: var(--space-4);
    padding-inline: calc(var(--space-4) + var(--card-safe-area-start))
      calc(var(--space-5) + var(--card-safe-area-end));
    border-inline-start: 4px solid var(--color-danger-subtle-text);
    font: var(--text-body);
  }
  .notice > span {
    flex-shrink: 0;
    font-size: 24px;
    color: var(--color-danger-subtle-text);
  }
  .notice > div {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  p {
    margin: var(--space-1) 0 0;
    color: var(--color-text-muted);
  }
</style>
