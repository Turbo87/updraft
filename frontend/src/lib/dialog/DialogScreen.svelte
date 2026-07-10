<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { m } from '$lib/paraglide/messages.js';
  import { getDialogControls } from './context';

  interface Props {
    /** The screen's title, shown in the header and used as the dialog's accessible name. */
    title: string;
    /** Show the Back control. Omit on a flow root, where only Close makes sense. */
    back?: boolean;
    /** The screen's body. */
    children: Snippet;
  }

  let { title, back = false, children }: Props = $props();

  // The shell (in the layout) owns the <dialog> and the history model; we render
  // this screen's header + body and drive Back/Close through its controls. This
  // component mounts fresh per screen, so nothing here has to reset on navigation.
  const controls = getDialogControls();

  // Each dialog screen is a route change, so move focus to the heading on mount:
  // it announces the new screen to a screen reader and would otherwise be stranded
  // on the (now unmounted) link that brought us here. Fresh mount per screen means
  // a one-shot onMount is enough — no page.url keying.
  let heading: HTMLHeadingElement | undefined;
  onMount(() => {
    heading?.focus();
  });
</script>

<div class="sheet">
  <header class="header">
    {#if back}
      <button type="button" class="icon" aria-label={m.dialog_back()} onclick={controls.back}>
        &#8592;
      </button>
    {/if}
    <h2 id={controls.headingId} bind:this={heading} tabindex="-1" class="title">{title}</h2>
    <button type="button" class="icon" aria-label={m.dialog_close()} onclick={controls.close}>
      &#10005;
    </button>
  </header>
  <div class="panel">
    {@render children()}
  </div>
</div>

<style>
  .sheet {
    display: flex;
    flex-direction: column;
    max-height: 85vh;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #e2e2e2;
  }

  .title {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 1.2rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The heading receives focus programmatically for screen readers; it is not an
     interactive control, so it gets no focus ring. */
  .title:focus {
    outline: none;
  }

  /* Glove/turbulence-friendly touch targets (~48px). */
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 3rem;
    height: 3rem;
    flex: none;
    border: none;
    border-radius: 0.5rem;
    background: none;
    color: #333;
    font: inherit;
    font-size: 1.1rem;
    line-height: 1;
    cursor: pointer;
  }

  .icon:hover {
    background: #efefef;
  }

  .panel {
    padding: 0.75rem 1rem 1rem;
    overflow-y: auto;
  }

  @media (max-width: 40rem) {
    .sheet {
      height: 100%;
      max-height: none;
    }
    .header {
      padding-top: max(0.5rem, env(safe-area-inset-top));
    }
    .panel {
      padding-bottom: max(1rem, env(safe-area-inset-bottom));
    }
  }

  @media (prefers-color-scheme: dark) {
    .header {
      border-bottom-color: #383838;
    }
    .icon {
      color: #dcdcdc;
    }
    .icon:hover {
      background: #2c2c2c;
    }
  }
</style>
