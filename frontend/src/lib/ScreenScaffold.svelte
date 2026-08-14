<script lang="ts">
  import type { Pathname } from '$app/types';
  import type { Snippet } from 'svelte';

  import { resolve } from '$app/paths';

  type BackTarget =
    | { backHref: Pathname; onBack?: never }
    | { backHref?: never; onBack: (event: MouseEvent) => void };

  type Props = BackTarget & {
    actions?: Snippet;
    backLabel: string;
    children: Snippet;
    title: string;
  };

  let { actions, backHref, backLabel, children, onBack, title }: Props = $props();
</script>

<div class="screen-scaffold">
  <header>
    {#if backHref}
      <a class="back-control" aria-label={backLabel} href={resolve(backHref)}>
        <span aria-hidden="true" class="i-mdi-arrow-left"></span>
      </a>
    {:else}
      <button class="back-control" aria-label={backLabel} type="button" onclick={onBack}>
        <span aria-hidden="true" class="i-mdi-arrow-left"></span>
      </button>
    {/if}
    <h1>{title}</h1>
  </header>

  <main>
    <div class="content">{@render children()}</div>
  </main>

  {#if actions}
    <footer>
      <div class="actions">{@render actions()}</div>
    </footer>
  {/if}
</div>

<style>
  .screen-scaffold {
    box-sizing: border-box;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    width: 100%;
    height: 100%;
    min-height: 0;
    background: var(--color-screen-surface);
    color: var(--color-text);
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding-block-start: var(--safe-area-top);
    padding-inline: calc(var(--space-2) + var(--safe-area-left))
      calc(var(--space-2) + var(--safe-area-right));
    border-block-end: 1px solid var(--color-border);
    background: var(--color-screen-surface);
  }

  .back-control {
    box-sizing: border-box;
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: var(--button-height-flight);
    height: var(--button-height-flight);
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
    text-decoration: none;
  }

  .back-control:active {
    background: var(--color-control-surface-pressed);
  }

  .back-control span {
    font-size: 1.75rem;
    line-height: 1;
  }

  h1 {
    min-width: 0;
    margin: 0;
    overflow: hidden;
    color: var(--color-text);
    font: var(--text-screen-title);
    letter-spacing: -0.01em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  main {
    min-height: 0;
    overflow-y: auto;
    background: var(--color-screen-surface-sunken);
  }

  .content {
    width: min(100%, 34rem);
    margin-inline: auto;
    padding: var(--space-4) calc(var(--space-5) + var(--safe-area-right)) var(--space-6)
      calc(var(--space-5) + var(--safe-area-left));
  }

  footer {
    padding: var(--space-3) calc(var(--space-4) + var(--safe-area-right))
      calc(var(--space-3) + var(--safe-area-bottom)) calc(var(--space-4) + var(--safe-area-left));
    border-block-start: 1px solid var(--color-border);
    background: var(--color-screen-surface);
  }

  .actions {
    display: flex;
    gap: var(--space-2);
    width: min(100%, 34rem);
    margin-inline: auto;
  }
</style>
