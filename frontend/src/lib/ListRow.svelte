<script lang="ts">
  import type { Pathname } from '$app/types';
  import type { Snippet } from 'svelte';

  import { resolve } from '$app/paths';

  type Detail =
    | { value: string; numeric?: boolean; trailing?: never }
    | { value?: never; numeric?: never; trailing: Snippet }
    | { value?: never; numeric?: never; trailing?: never };

  type BaseProps = Detail & {
    label: string;
    icon?: string;
    disabled?: boolean;
    class?: string;
  };

  type Props = BaseProps &
    ({ href: Pathname; size: 'large' } | { href?: never; size?: 'standard' | 'large' });

  let {
    label,
    icon,
    value,
    numeric = false,
    trailing,
    href,
    size = 'standard',
    disabled = false,
    class: className,
  }: Props = $props();

  let navigates = $derived(href !== undefined);
</script>

{#snippet content()}
  {#if icon}
    <span aria-hidden="true" class={[icon, 'icon']}></span>
  {/if}
  <span class="label">{label}</span>
  {#if value || trailing || (navigates && !disabled)}
    <span class="end">
      {#if value}
        <span class:numeric class="value">{value}</span>
      {:else if trailing}
        <span class="trailing">{@render trailing()}</span>
      {/if}
      {#if navigates && !disabled}
        <span aria-hidden="true" class="i-mdi-chevron-right chevron"></span>
      {/if}
    </span>
  {/if}
{/snippet}

{#if href && !disabled}
  <a href={resolve(href)} class={['list-row', size, className]}>
    {@render content()}
  </a>
{:else}
  <div aria-disabled={disabled || undefined} class={['list-row', size, { disabled }, className]}>
    {@render content()}
  </div>
{/if}

<style>
  .list-row {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    min-height: var(--list-row-min-height);
    padding: var(--space-2) var(--space-5);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    background: var(--color-card-surface);
    color: var(--color-text);
  }

  a {
    text-decoration: none;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  .standard {
    --list-row-min-height: var(--target-min);
  }

  .large {
    --list-row-min-height: var(--target-flight);
  }

  a:active {
    background: var(--color-control-surface-pressed);
  }

  a:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }

  .disabled {
    background: var(--color-disabled-surface);
    color: var(--color-disabled-text);
  }

  .icon {
    flex: 0 0 auto;
    color: var(--color-text-muted);
    font-size: 1.5rem;
    line-height: 1;
  }

  .label {
    min-width: 0;
    font: var(--text-row-label);
  }

  .end {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    min-width: 0;
    margin-inline-start: auto;
  }

  .value {
    flex: 0 1 auto;
    color: var(--color-text-muted);
    font: var(--text-row-detail);
    text-align: end;
  }

  .value.numeric {
    color: var(--color-value-text);
    font: var(--text-row-value);
    font-variant-numeric: tabular-nums;
  }

  .disabled .value,
  .disabled .icon {
    color: inherit;
  }

  .trailing {
    display: inline-flex;
    align-items: center;
  }

  .chevron {
    flex: 0 0 auto;
    color: var(--color-text-muted);
    font-size: 1.75rem;
    line-height: 1;
  }

  @media (prefers-reduced-motion: reduce) {
    a {
      transition: none;
    }
  }
</style>
