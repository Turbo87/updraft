import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';

import '../app.css';

import TrafficSymbol from './TrafficSymbol.svelte';

afterEach(() => document.documentElement.style.removeProperty('--traffic-symbol-size'));

function renderedSymbol(): HTMLElement {
  let symbol = document.querySelector<HTMLElement>('.traffic-symbol');
  if (!symbol) throw new Error('Traffic symbol did not render.');
  return symbol;
}

describe('TrafficSymbol.svelte', () => {
  it('applies the track, alarm level, and stale state', () => {
    render(TrafficSymbol, {
      alarmLevel: 'important',
      stale: true,
      trackDegrees: 118,
      trafficType: 'towPlane',
    });

    let symbol = renderedSymbol();

    expect(symbol).toHaveAttribute('aria-hidden', 'true');
    expect(symbol).toHaveClass('important', 'stale');
    expect(symbol).toHaveStyle({ transform: 'rotate(118deg)' });
    expect(getComputedStyle(symbol).opacity).toBe('0.45');
  });

  it('does not rotate balloons', () => {
    render(TrafficSymbol, {
      trackDegrees: 45,
      trafficType: 'balloon',
    });

    expect(renderedSymbol().style.transform).toBe('');
  });

  it('uses one em as its default size', () => {
    render(TrafficSymbol, { trafficType: 'glider' });

    let styles = getComputedStyle(renderedSymbol());

    expect(styles.width).toBe(styles.fontSize);
    expect(styles.height).toBe(styles.fontSize);
  });

  it('accepts a size from its parent', () => {
    document.documentElement.style.setProperty('--traffic-symbol-size', '40px');
    render(TrafficSymbol, { trafficType: 'glider' });

    let styles = getComputedStyle(renderedSymbol());

    expect(styles.width).toBe('40px');
    expect(styles.height).toBe('40px');
  });
});
