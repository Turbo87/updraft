<script lang="ts">
  import type { Snippet } from 'svelte';
  import { tick } from 'svelte';
  import type { Pathname } from '$app/types';
  import { page } from '$app/state';
  import { goto, afterNavigate } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { reduceDialogHistory, closeAction, INITIAL, type DialogHistory } from './navigation';

  interface Props {
    /**
     * The "closed" URL (e.g. the map). Any path *below* it (`${base}/…`) is a
     * dialog screen; landing back on `base` closes the dialog. Each screen is a
     * `+page.svelte`; the layout renders the current one via `children`.
     */
    base: Pathname;
    /** Title of the current screen — typically `page.data.title`. */
    title?: string;
    /** Parent URL to step up to, or `null` at a flow root — typically `page.data.back`. */
    back?: Pathname | null;
    /** Accessible label for the Back control. The consumer localizes it. */
    backLabel?: string;
    /** Accessible label for the Close control. The consumer localizes it. */
    closeLabel?: string;
    /** The current screen's content (the routed `+page`). */
    children: Snippet;
  }

  let {
    base,
    title = '',
    back = null,
    backLabel = 'Back',
    closeLabel = 'Close',
    children,
  }: Props = $props();

  // A path is a dialog screen when it lives below `base`. Normalising the prefix
  // lets `base` be the site root ("/") as well as a sub-tree ("/foo").
  function isDialogPath(p: string | undefined): boolean {
    let prefix = base.endsWith('/') ? base : `${base}/`;
    return !!p && p !== base && p.startsWith(prefix);
  }
  let isDialog = $derived(isDialogPath(page.url.pathname));

  const headingId = $props.id();

  let el: HTMLDialogElement | undefined = $state();
  let heading: HTMLHeadingElement | undefined = $state();
  let panel: HTMLDivElement | undefined = $state();

  // History model (Option 1): drilling deeper pushes an entry, so the hardware /
  // gesture Back button steps up one level; closing collapses the whole run with
  // history.go(-depth) so Back can't re-open a dismissed dialog. Depth is tracked
  // from SvelteKit's signed popstate `delta` (see navigation.ts). Read only in
  // event handlers, so a plain variable is enough.
  let history_: DialogHistory = INITIAL;
  let closing = false; // a collapse is in flight — ignore repeat close/Esc/backdrop
  let finishColdClose = false; // after collapsing cold entries, replace the first with `base`
  let replacing = false; // the next navigation is an internal replaceState (goUp)
  afterNavigate((nav) => {
    let onDialog = isDialogPath(nav.to?.url?.pathname);
    history_ = reduceDialogHistory(history_, {
      type: nav.type,
      fromDialog: isDialogPath(nav.from?.url?.pathname),
      toDialog: onDialog,
      delta: nav.type === 'popstate' ? nav.delta : undefined,
      isReplace: replacing,
    });
    replacing = false;
    if (finishColdClose && onDialog) {
      // The drilled cold entries are collapsed back to the first dialog entry;
      // replace it with the map to finish closing.
      finishColdClose = false;
      goto(resolve(base), { replaceState: true });
    } else if (!onDialog) {
      closing = false; // fully closed, back on the map
    }
  });

  // Open/close the native <dialog> to match the route. showModal() gives us the
  // focus trap, inert background, Esc and ::backdrop for free. The consumer keeps
  // the surface behind this component (e.g. a map) mounted across navigations, so
  // opening a dialog never disturbs it.
  $effect(() => {
    if (!el) return;
    if (isDialog) {
      if (!el.open) el.showModal();
    } else if (el.open) {
      el.close();
    }
  });

  // Treat each dialog route like the route change it is: move focus to the
  // (freshly-keyed) heading so screen readers announce the new screen, and reset
  // the panel scroll.
  $effect(() => {
    void page.url.pathname;
    if (!isDialog) return;
    void tick().then(() => {
      if (!isDialog) return;
      heading?.focus();
      panel?.scrollTo(0, 0);
    });
  });

  // Close = dismiss the whole dialog without leaving drilled entries that Back
  // could re-open. Warm (map beneath): collapse the run with go(-depth). Cold +
  // drilled: pop back to the first dialog entry, then (in afterNavigate) replace
  // it with the map. Cold single entry: replace it with the map directly.
  function close() {
    if (closing) return; // guard against double-tap during the async collapse
    closing = true;
    let action = closeAction(history_);
    if (action.kind === 'back') {
      window.history.go(-action.steps);
    } else if (action.kind === 'collapse') {
      finishColdClose = true;
      window.history.go(-action.steps);
    } else {
      goto(resolve(base), { replaceState: true });
    }
  }

  // Back = step up exactly one level. If we drilled here, the parent is a real
  // entry beneath us, so a popstate mirrors the hardware Back button. At the first
  // dialog entry (a cold-loaded screen), synthesize the parent with a replace.
  function goUp() {
    if (history_.depth > 1) {
      window.history.back();
    } else if (back) {
      replacing = true;
      goto(resolve(back), { replaceState: true });
    }
  }

  // Esc closes the whole dialog. We prevent the native close and drive it through
  // close() instead, so the URL stays in sync with what's on screen.
  function onCancel(event: Event) {
    event.preventDefault();
    close();
  }

  // Backdrop light-dismiss, guarded so a drag ending on the backdrop won't close.
  let pressedBackdrop = false;
  function onPointerDown(event: PointerEvent) {
    pressedBackdrop = event.target === el;
  }
  function onBackdropClick(event: MouseEvent) {
    if (event.target === el && pressedBackdrop) close();
    pressedBackdrop = false;
  }
</script>

<dialog
  bind:this={el}
  class="dialog"
  aria-labelledby={headingId}
  oncancel={onCancel}
  onpointerdown={onPointerDown}
  onclick={onBackdropClick}
>
  {#if isDialog}
    <div class="sheet">
      <header class="header">
        {#if back}
          <button type="button" class="icon" aria-label={backLabel} onclick={goUp}>
            &#8592;
          </button>
        {/if}
        {#key page.url.pathname}
          <h2 id={headingId} bind:this={heading} tabindex="-1" class="title">{title}</h2>
        {/key}
        <button type="button" class="icon" aria-label={closeLabel} onclick={close}>
          &#10005;
        </button>
      </header>
      <div class="panel" bind:this={panel}>
        {@render children()}
      </div>
    </div>
  {/if}
</dialog>

<style>
  /* Centered card on large screens → fullscreen on small screens. */
  .dialog {
    width: min(92vw, 30rem);
    max-width: 92vw;
    max-height: 85vh;
    padding: 0;
    border: none;
    border-radius: 0.75rem;
    background: #fff;
    color: #141414;
    box-shadow:
      0 10px 40px rgba(0, 0, 0, 0.25),
      0 2px 8px rgba(0, 0, 0, 0.1);
    overflow: hidden;
  }

  .dialog::backdrop {
    background: rgba(0, 0, 0, 0.5);
  }

  @media (max-width: 40rem) {
    .dialog {
      inset: 0;
      width: 100%;
      max-width: none;
      height: 100dvh;
      max-height: none;
      margin: 0;
      border-radius: 0;
    }
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

  @media (prefers-reduced-motion: no-preference) {
    .dialog[open] {
      animation: dialog-in 160ms ease-out;
    }
  }

  @keyframes dialog-in {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }
  }

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

  .dialog :global(a:focus-visible),
  .dialog :global(button:focus-visible),
  .dialog :global(input:focus-visible) {
    outline: 2px solid #1d4ed8;
    outline-offset: 2px;
  }

  @media (prefers-color-scheme: dark) {
    .dialog {
      background: #1e1e1e;
      color: #f2f2f2;
    }
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
