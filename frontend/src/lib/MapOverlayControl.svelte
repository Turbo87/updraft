<script lang="ts">
  import type { Pathname } from '$app/types';

  import { resolve } from '$app/paths';

  type CommonProps = {
    icon: string;
    label: string;
    class?: string;
  };

  type Props = CommonProps &
    ({ href: Pathname; onClick?: never } | { href?: never; onClick: () => void });

  let { icon, label, href, onClick, class: className }: Props = $props();
</script>

{#snippet content()}
  <span aria-hidden="true" class={[icon, 'icon']}></span>
{/snippet}

{#if href}
  <a aria-label={label} class={['map-overlay-control', className]} href={resolve(href)}>
    {@render content()}
  </a>
{:else}
  <button
    type="button"
    aria-label={label}
    class={['map-overlay-control', className]}
    onclick={onClick}
  >
    {@render content()}
  </button>
{/if}

<style>
  .map-overlay-control {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--target-flight);
    height: var(--target-flight);
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--color-map-overlay-scrim);
    box-shadow: var(--shadow-overlay);
    color: var(--color-map-overlay-text);
    cursor: pointer;
    text-decoration: none;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  .map-overlay-control:active {
    background: var(--color-map-overlay-scrim-pressed);
  }

  .map-overlay-control:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }

  .icon {
    width: 1.75rem;
    height: 1.75rem;
    font-size: 1.75rem;
  }

  @media (prefers-reduced-motion: reduce) {
    .map-overlay-control {
      transition: none;
    }
  }
</style>
