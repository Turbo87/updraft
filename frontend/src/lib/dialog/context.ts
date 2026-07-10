import { createContext } from 'svelte';

// The persistent RouteDialog shell owns the <dialog>, the history model, and the
// Back/Close controls; each routed DialogScreen renders its own header + body and
// reaches the shell's controls through this context.

export interface DialogControls {
  /** Dismiss the whole dialog run and return to the map. */
  close: () => void;
  /** Step up exactly one level. */
  back: () => void;
  /** id the shell's <dialog> points `aria-labelledby` at; goes on the screen's heading. */
  headingId: string;
}

export const [getDialogControls, setDialogControls] = createContext<DialogControls>();
