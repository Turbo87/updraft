import { describe, expect, it } from 'vitest';
import { ApplicationState } from './state.svelte';
import type { OwnshipPosition } from './generated/OwnshipPosition';

const aachen: OwnshipPosition = {
  location: { latitude: 50.823, longitude: 6.186 },
  track: 45,
};

const cologne: OwnshipPosition = {
  location: { latitude: 50.866, longitude: 7.143 },
  track: null,
};

describe('ApplicationState', () => {
  it('starts empty and connecting', () => {
    let state = new ApplicationState();
    expect(state.position).toBeNull();
    expect(state.streamStatus).toBe('connecting');
    expect(state.lastUpdatedAt).toBeNull();
  });

  it('seeds from a snapshot', () => {
    let state = new ApplicationState();
    state.applySnapshot({ position: aachen });
    expect(state.position).toEqual(aachen);
    expect(state.lastUpdatedAt).not.toBeNull();
  });

  it('applies position changes in order', () => {
    let state = new ApplicationState();
    state.applySnapshot({ position: null });
    state.applyChanges([
      { flight: { position_changed: aachen } },
      { flight: { position_changed: cologne } },
    ]);
    expect(state.position).toEqual(cologne);
  });

  it('replaces state on a fresh snapshot after reconnect', () => {
    let state = new ApplicationState();
    state.applyChanges([{ flight: { position_changed: aachen } }]);
    state.applySnapshot({ position: cologne });
    expect(state.position).toEqual(cologne);
  });
});
