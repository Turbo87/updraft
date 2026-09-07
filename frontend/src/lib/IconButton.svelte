<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Props = Omit<HTMLButtonAttributes, 'children' | 'aria-label' | 'aria-labelledby'> & {
    icon: string;
    label: string;
  };

  let { icon, label, class: className, type = 'button', ...attributes }: Props = $props();
</script>

<button {...attributes} {type} aria-label={label} class={className}>
  <span aria-hidden="true" class={[icon, 'icon']}></span>
</button>

<style>
  button {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: var(--target-min);
    height: var(--target-min);
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--color-control-surface-raised);
    color: var(--color-text);
    cursor: pointer;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  button:active:not(:disabled) {
    background: var(--color-control-surface-raised-pressed);
  }

  button:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }

  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .icon {
    width: 1.5rem;
    height: 1.5rem;
    font-size: 1.5rem;
  }

  @media (prefers-reduced-motion: reduce) {
    button {
      transition: none;
    }
  }
</style>
