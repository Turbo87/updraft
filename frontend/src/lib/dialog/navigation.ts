// Dialog history model (Option 1): drilling deeper pushes a history entry, so the
// browser/OS Back button steps up one level; closing collapses the whole run with
// `history.go(-depth)` so Back never re-opens a dismissed dialog.
//
// The only hard part — knowing the depth across arbitrary back/forward jumps — is
// solved with SvelteKit's signed `delta` on popstate navigations. This module is
// the pure reducer that turns navigation events into { depth, hasOpener }, kept
// free of Svelte/DOM so every case can be unit-tested in isolation.

export interface DialogHistory {
  /** Levels into the dialog. 0 = on the map (no dialog); >= 1 = drilled that deep. */
  depth: number;
  /**
   * Whether a real map history entry exists beneath the dialog — i.e. we entered
   * the dialog by navigating from within the app, rather than cold-loading a
   * dialog URL directly (deep link / refresh), which has nothing beneath it.
   */
  hasOpener: boolean;
}

export interface NavEvent {
  /** SvelteKit navigation type. */
  type: 'enter' | 'link' | 'goto' | 'form' | 'popstate';
  /** Was the previous route a dialog route? (false for the map or a cold load.) */
  fromDialog: boolean;
  /** Is the destination a dialog route? */
  toDialog: boolean;
  /** Signed step count; only present (and only meaningful) for `popstate`. */
  delta?: number;
  /**
   * Whether this navigation used `replaceState` (no new history entry). We only
   * ever replace between dialog screens to step *up* one level (goUp on a
   * cold-loaded screen), so a replace moves one level shallower rather than
   * adding depth.
   */
  isReplace?: boolean;
}

export const INITIAL: DialogHistory = { depth: 0, hasOpener: false };

/** Fold a navigation event into the dialog-history state. Pure. */
export function reduceDialogHistory(state: DialogHistory, event: NavEvent): DialogHistory {
  // Landing on a non-dialog route (the map) means the dialog is closed.
  if (!event.toDialog) return { depth: 0, hasOpener: false };

  switch (event.type) {
    // Cold load / refresh straight onto a dialog URL: there is no map entry
    // beneath us, so closing must synthesize a return to the map.
    case 'enter':
      return { depth: 1, hasOpener: false };

    // Back/forward: `delta` is the exact signed number of entries moved, so we
    // never have to guess direction. Clamp at 1 while still inside the dialog.
    case 'popstate':
      return { ...state, depth: Math.max(1, state.depth + (event.delta ?? 0)) };

    // Forward navigation (link / goto / GET form).
    default:
      // A replace between dialog screens steps up one level without adding an entry.
      if (event.isReplace) return { ...state, depth: Math.max(1, state.depth - 1) };
      // Opening from the map: this is the first dialog entry, with the map beneath.
      if (!event.fromDialog) return { depth: 1, hasOpener: true };
      // Drilling one level deeper.
      return { ...state, depth: state.depth + 1 };
  }
}

export type CloseAction =
  // Warm (map beneath): pop the whole dialog run back to the map entry.
  | { kind: 'back'; steps: number }
  // Cold + drilled: pop back to the first dialog entry (steps = depth - 1); the
  // caller then replaces that entry with the map.
  | { kind: 'collapse'; steps: number }
  // Cold, single entry: replace it with the map.
  | { kind: 'replace' };

/** How to dismiss the dialog given the current history state, without leaving
 *  drilled entries behind for the Back button to re-open. */
export function closeAction(state: DialogHistory): CloseAction {
  if (state.hasOpener && state.depth > 0) return { kind: 'back', steps: state.depth };
  if (state.depth > 1) return { kind: 'collapse', steps: state.depth - 1 };
  return { kind: 'replace' };
}
