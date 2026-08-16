<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Props = Omit<HTMLButtonAttributes, 'children'> & {
    children: Snippet;
    variant?: 'primary' | 'secondary' | 'destructive' | 'destructive-outline';
    size?: 'standard' | 'large';
    loading?: boolean;
  };

  let {
    children,
    variant = 'primary',
    size = 'standard',
    loading = false,
    disabled = false,
    class: className,
    type = 'button',
    ...attributes
  }: Props = $props();
</script>

<button
  {...attributes}
  {type}
  disabled={disabled || loading}
  aria-busy={loading || undefined}
  class={[variant, size, { loading }, className]}
>
  {#if loading}
    <span aria-hidden="true" class="i-mdi-loading loading-icon"></span>
  {/if}
  {@render children()}
</button>

<style>
  button {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    height: var(--button-height);
    padding-inline: var(--space-4);
    border: var(--button-border, 0);
    border-radius: var(--radius-control);
    background: var(--button-surface);
    color: var(--button-text);
    font: var(--text-button);
    cursor: pointer;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard);
  }

  .standard {
    --button-height: var(--target-min);
  }

  .large {
    --button-height: var(--button-height-flight);

    font: var(--text-button-large);
  }

  .primary {
    --button-surface: var(--color-action-primary-surface);
    --button-text: var(--color-action-primary-text);
    --button-pressed-surface: light-dark(var(--color-blue-600), var(--color-blue-300));
  }

  .secondary {
    --button-surface: var(--color-action-secondary-surface);
    --button-text: var(--color-action-secondary-text);
    --button-pressed-surface: var(--color-action-secondary-surface-pressed);
  }

  .destructive {
    --button-surface: var(--color-action-destructive-surface);
    --button-text: var(--color-action-destructive-text);
    --button-pressed-surface: light-dark(var(--color-red-600), var(--color-red-300));
  }

  .destructive-outline {
    --button-surface: transparent;
    --button-text: var(--color-action-destructive-surface);
    --button-border: 1px solid var(--color-action-destructive-surface);
    --button-pressed-surface: var(--color-danger-subtle-surface);
  }

  button:active:not(:disabled) {
    background: var(--button-pressed-surface);
  }

  button:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }

  button:disabled:not(.loading) {
    opacity: 0.45;
    cursor: not-allowed;
  }

  button.loading {
    opacity: 0.8;
    cursor: wait;
  }

  .loading-icon {
    width: 1.5rem;
    height: 1.5rem;
    animation: button-loading-spin 900ms linear infinite;
  }

  @keyframes button-loading-spin {
    to {
      transform: rotate(1turn);
    }
  }

  @keyframes button-loading-pulse {
    50% {
      opacity: 0.4;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    button {
      transition: none;
    }

    .loading-icon {
      animation: button-loading-pulse 1.2s ease-in-out infinite;
    }
  }
</style>
