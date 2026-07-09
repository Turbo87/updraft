import { describe, expect, it } from 'vitest';
import { ApplicationState } from './state.svelte';

describe('ApplicationState', () => {
  it('replaces its position from a snapshot', () => {
    let state = new ApplicationState();

    state.applySnapshot({
      position: {
        location: { latitude: 50.823, longitude: 6.186 },
        track: 45,
      },
    });

    expect(state.position).toEqual({
      location: { latitude: 50.823, longitude: 6.186 },
      track: 45,
    });
  });

  it('applies position changes in order', () => {
    let state = new ApplicationState();

    state.applyChanges([
      {
        flight: {
          position_changed: {
            location: { latitude: 50.823, longitude: 6.186 },
            track: null,
          },
        },
      },
      {
        flight: {
          position_changed: {
            location: { latitude: 50.824, longitude: 6.187 },
            track: 90,
          },
        },
      },
    ]);

    expect(state.position).toEqual({
      location: { latitude: 50.824, longitude: 6.187 },
      track: 90,
    });
  });
});
