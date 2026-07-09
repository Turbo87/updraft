import { describe, expect, it } from 'vitest';
import {
  reduceDialogHistory,
  closeAction,
  INITIAL,
  type DialogHistory,
  type NavEvent,
} from './navigation';

// Event builders for the navigation types we care about.
const open = (): NavEvent => ({ type: 'link', fromDialog: false, toDialog: true }); // map → dialog
const drill = (): NavEvent => ({ type: 'link', fromDialog: true, toDialog: true }); // dialog → deeper
const tap = (): NavEvent => ({ type: 'goto', fromDialog: false, toDialog: true }); // map tap → what's-here
const enter = (): NavEvent => ({ type: 'enter', fromDialog: false, toDialog: true }); // cold load
const pop = (delta: number, toDialog = true): NavEvent => ({
  type: 'popstate',
  fromDialog: true,
  toDialog,
  delta,
});
const upReplace = (): NavEvent => ({
  type: 'goto',
  fromDialog: true,
  toDialog: true,
  isReplace: true,
}); // goUp on a cold screen
const toMap = (type: NavEvent['type'] = 'popstate', delta = -1): NavEvent => ({
  type,
  fromDialog: true,
  toDialog: false,
  delta: type === 'popstate' ? delta : undefined,
});

// Replay a sequence of events from the initial state.
const run = (...events: NavEvent[]): DialogHistory => events.reduce(reduceDialogHistory, INITIAL);

describe('reduceDialogHistory', () => {
  it('starts on the map (closed)', () => {
    expect(INITIAL).toEqual({ depth: 0, hasOpener: false });
  });

  it('opening from the map is depth 1 with an opener', () => {
    expect(run(open())).toEqual({ depth: 1, hasOpener: true });
  });

  it('drilling deeper increments depth and keeps the opener', () => {
    expect(run(open(), drill())).toEqual({ depth: 2, hasOpener: true });
    expect(run(open(), drill(), drill())).toEqual({ depth: 3, hasOpener: true });
  });

  it('the map tap jump counts as one push, not one-per-path-segment', () => {
    // tap → /whats-here/@c (2 segments) then drill → /whats-here/@c/feature (3 segments)
    // is two history pushes, so depth is 2 — the property naive segment-counting gets wrong.
    expect(run(tap(), drill())).toEqual({ depth: 2, hasOpener: true });
  });

  it('back (popstate -1) steps up exactly one level', () => {
    expect(run(open(), drill(), drill(), pop(-1))).toEqual({ depth: 2, hasOpener: true });
  });

  it('a multi-step back jump uses the signed delta', () => {
    expect(run(open(), drill(), drill(), pop(-2))).toEqual({ depth: 1, hasOpener: true });
  });

  it('forward (popstate +1) steps back down', () => {
    expect(run(open(), drill(), drill(), pop(-2), pop(1))).toEqual({ depth: 2, hasOpener: true });
  });

  it('never lets depth drop below 1 while still inside the dialog', () => {
    expect(run(open(), drill(), pop(-5))).toEqual({ depth: 1, hasOpener: true });
  });

  it('landing back on the map closes, whatever the navigation type', () => {
    expect(run(open(), drill(), toMap('popstate', -2))).toEqual(INITIAL);
    expect(run(open(), drill(), toMap('goto'))).toEqual(INITIAL); // ✕ / Esc / close()
  });

  it('reopening after closing starts a fresh depth-1 run', () => {
    expect(run(open(), drill(), toMap(), open())).toEqual({ depth: 1, hasOpener: true });
  });

  describe('cold deep-link (no map beneath)', () => {
    it('entering a dialog URL directly is depth 1 with no opener', () => {
      expect(run(enter())).toEqual({ depth: 1, hasOpener: false });
    });

    it('drilling from a cold-loaded screen keeps hasOpener false', () => {
      expect(run(enter(), drill())).toEqual({ depth: 2, hasOpener: false });
    });

    it('an in-app open after a cold load restores the opener', () => {
      expect(run(enter(), toMap(), open())).toEqual({ depth: 1, hasOpener: true });
    });

    it('stepping up via replace moves one level shallower without adding depth', () => {
      // Cold-load a leaf, drill deeper, then goUp (replace) back one level.
      expect(run(enter(), drill(), upReplace())).toEqual({ depth: 1, hasOpener: false });
    });

    it('stepping up via replace at the first entry stays at depth 1', () => {
      expect(run(enter(), upReplace())).toEqual({ depth: 1, hasOpener: false });
    });
  });

  it('ignores an unused delta on forward navigations', () => {
    let weird: NavEvent = { type: 'link', fromDialog: true, toDialog: true, delta: 99 };
    expect(run(open(), weird)).toEqual({ depth: 2, hasOpener: true });
  });
});

describe('closeAction', () => {
  it('collapses the whole run back to the map when opened in-app', () => {
    expect(closeAction(run(open(), drill(), drill()))).toEqual({ kind: 'back', steps: 3 });
    expect(closeAction(run(tap(), drill()))).toEqual({ kind: 'back', steps: 2 });
  });

  it('replaces to the map on a cold single-entry dialog (nothing to pop)', () => {
    expect(closeAction(run(enter()))).toEqual({ kind: 'replace' });
  });

  it('collapses drilled cold entries: pop to the first entry, then the caller replaces it', () => {
    // Cold-load a list then drill to a feature: closing must remove BOTH so Back
    // can't re-open the dismissed dialog.
    expect(closeAction(run(enter(), drill()))).toEqual({ kind: 'collapse', steps: 1 });
    expect(closeAction(run(enter(), drill(), drill()))).toEqual({ kind: 'collapse', steps: 2 });
  });

  it('back steps always match the current depth after up/down navigation', () => {
    expect(closeAction(run(open(), drill(), drill(), pop(-1)))).toEqual({ kind: 'back', steps: 2 });
  });

  it('replaces when there is nothing open', () => {
    expect(closeAction(INITIAL)).toEqual({ kind: 'replace' });
  });
});
