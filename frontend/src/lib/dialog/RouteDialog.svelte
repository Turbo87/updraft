<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Pathname } from '$app/types';
  import { page } from '$app/state';
  import { goto, afterNavigate, replaceState } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { setDialogControls } from './context';

  interface DialogEntry {
    /** Levels into the dialog. 0 = closed (on the map); >= 1 = drilled that deep. */
    depth: number;
    /**
     * Whether a real map entry sits beneath the first dialog entry — i.e. we opened
     * the dialog from within the app, rather than cold-loading a dialog URL directly
     * (deep link / refresh), which has nothing beneath it.
     */
    hasOpener: boolean;
  }

  interface Props {
    /**
     * The "closed" URL (e.g. the map). Any path *below* it (`${base}/…`) is a
     * dialog screen; landing back on `base` closes the dialog. Each screen is a
     * `+page.svelte` wrapping its content in `DialogScreen`; this shell renders
     * the current one via `children`.
     */
    base: Pathname;
    /** The current screen's content (the routed `+page`). */
    children: Snippet;
  }

  let { base, children }: Props = $props();

  // A path is a dialog screen when it lives below `base`. Normalising the prefix
  // lets `base` be the site root ("/") as well as a sub-tree ("/foo").
  function isDialogPath(p: string | undefined): boolean {
    let prefix = base.endsWith('/') ? base : `${base}/`;
    return !!p && p !== base && p.startsWith(prefix);
  }
  let isDialog = $derived(isDialogPath(page.url.pathname));

  const headingId = $props.id();
  let el: HTMLDialogElement | undefined = $state();

  // History model: drilling deeper pushes an entry, so the hardware / gesture Back
  // button steps up one level; closing collapses the whole run with go(-depth) so
  // Back can't re-open a dismissed dialog. Each entry carries its own depth in
  // page.state, which the browser restores on back/forward — so we read depth off
  // the current entry and only compute it for a fresh forward/cold entry. Read
  // only in event handlers, so a plain variable is enough.
  let current: DialogEntry = { depth: 0, hasOpener: false };
  let closing = false; // a collapse is in flight — ignore repeat close/Esc/backdrop
  let finishColdClose = false; // after collapsing cold entries, replace the first with `base`

  afterNavigate((nav) => {
    if (!isDialogPath(page.url.pathname)) {
      current = { depth: 0, hasOpener: false };
      closing = false; // fully closed, back on the map
      return;
    }

    if (page.state.dialog) {
      // The browser restored this entry's state on back/forward (or we stamped it
      // on a prior visit): read it, don't recompute.
      current = page.state.dialog;
    } else {
      // A fresh forward push or cold load: compute the depth and stamp it onto the
      // entry so a later back/forward restores it. Idempotent — the stamp itself
      // never lands here again because page.state.dialog is then set. Drilling
      // deeper keeps the opener and adds a level; the first entry is depth 1, with
      // an opener beneath unless we cold-loaded straight onto it ('enter').
      current = isDialogPath(nav.from?.url?.pathname)
        ? { depth: current.depth + 1, hasOpener: current.hasOpener }
        : { depth: 1, hasOpener: nav.type !== 'enter' };

      replaceState(page.url, { dialog: current });
    }

    if (finishColdClose) {
      // The drilled cold entries are collapsed back to the first dialog entry;
      // replace it with the map to finish closing.
      finishColdClose = false;
      goto(resolve(base), { replaceState: true });
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

  // Close = dismiss the whole dialog without leaving drilled entries that Back
  // could re-open. Warm (map beneath): collapse the run with go(-depth). Cold +
  // drilled: pop back to the first dialog entry, then (in afterNavigate) replace
  // it with the map. Cold single entry: replace it with the map directly.
  function close() {
    if (closing) return; // guard against double-tap during the async collapse
    closing = true;
    if (current.hasOpener && current.depth > 0) {
      // Warm: the map sits `depth` entries back, so pop the whole run.
      window.history.go(-current.depth);
    } else if (current.depth > 1) {
      // Cold + drilled: pop back to the first (cold) entry, then replace it below.
      finishColdClose = true;
      window.history.go(-current.depth + 1);
    } else {
      // Cold single entry: nothing to pop, just replace it with the map.
      goto(resolve(base), { replaceState: true });
    }
  }

  // Back = step up exactly one level. When a real entry sits beneath us (a deeper
  // dialog entry or the map) a plain history.back() mirrors the hardware Back
  // button. On a cold single entry there is nothing beneath, so synthesize the
  // return to the map with a replace.
  function goUp() {
    // A real entry sits beneath us (a deeper dialog entry, or the map) → step up.
    if (current.hasOpener || current.depth > 1) {
      window.history.back();
    } else {
      goto(resolve(base), { replaceState: true });
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

  setDialogControls({ close, back: goUp, headingId });
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
    {@render children()}
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
  }
</style>
