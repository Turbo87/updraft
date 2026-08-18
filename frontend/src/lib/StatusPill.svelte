<script lang="ts">
  type Indicator = { loading: true; icon?: never } | { loading?: false; icon?: string };

  type Props = Indicator & {
    label: string;
    tone?: 'normal' | 'caution' | 'success' | 'danger-subtle' | 'warning' | 'danger';
    class?: string;
  };

  let { label, tone = 'normal', icon, loading = false, class: className }: Props = $props();
</script>

<span class={['status-pill', tone, className]}>
  {#if loading || icon}
    <span aria-hidden="true" class="indicator-slot">
      {#if loading}
        <span class="i-mdi-loading indicator loading"></span>
      {:else}
        <span class={[icon, 'indicator']}></span>
      {/if}
    </span>
  {/if}
  {label}
</span>

<style>
  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.375em;
    padding: 0.3125em 0.75em;
    border-radius: 999px;
    background: var(--status-pill-surface);
    color: var(--status-pill-text);
    font-family: var(--font-ui);
    font-size: var(--status-pill-font-size, 1rem);
    font-weight: 600;
    line-height: 1;
    vertical-align: middle;
    white-space: nowrap;
  }

  .normal {
    --status-pill-surface: var(--color-normal-surface);
    --status-pill-text: var(--color-normal-text);
  }

  .caution {
    --status-pill-surface: var(--color-caution-surface);
    --status-pill-text: var(--color-caution-text);
  }

  .success {
    --status-pill-surface: var(--color-success-surface);
    --status-pill-text: var(--color-success-text);
  }

  .danger-subtle {
    --status-pill-surface: var(--color-danger-subtle-surface);
    --status-pill-text: var(--color-danger-subtle-text);
  }

  .warning {
    --status-pill-surface: var(--color-warning-surface);
    --status-pill-text: var(--color-warning-text);
  }

  .danger {
    --status-pill-surface: var(--color-danger-surface);
    --status-pill-text: var(--color-danger-text);
  }

  .indicator-slot {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: 1em;
    height: 1em;
  }

  .indicator {
    font-size: 1em;
    line-height: 1;
  }

  .loading {
    animation: status-pill-loading-spin 900ms linear infinite;
  }

  @keyframes status-pill-loading-spin {
    to {
      transform: rotate(1turn);
    }
  }

  @keyframes status-pill-loading-pulse {
    50% {
      opacity: 0.4;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .loading {
      animation: status-pill-loading-pulse 1.2s ease-in-out infinite;
    }
  }
</style>
