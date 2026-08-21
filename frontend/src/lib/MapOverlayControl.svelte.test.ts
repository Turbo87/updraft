import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import MapOverlayControl from './MapOverlayControl.svelte';

describe('MapOverlayControl.svelte', () => {
  it('renders an icon-only route link as a round 56-pixel target', async () => {
    render(MapOverlayControl, {
      href: '/settings',
      icon: 'i-mdi-menu',
      label: 'Settings',
    });

    let link = page.getByRole('link', { name: 'Settings' });
    await expect.element(link).toHaveAttribute('href', '/settings');

    let style = getComputedStyle(link.element());
    expect(style.width).toBe('56px');
    expect(style.height).toBe('56px');
    expect(style.borderRadius).toBe('50%');
    expect(style.getPropertyValue('-webkit-tap-highlight-color')).toBe('rgba(0, 0, 0, 0)');
    expect(link.element().querySelector('.i-mdi-menu')).not.toBeNull();
  });

  it('invokes the callback button action', async () => {
    let onClick = vi.fn();

    render(MapOverlayControl, {
      icon: 'i-mdi-crosshairs-gps',
      label: 'Return to position',
      onClick,
    });

    await page.getByRole('button', { name: 'Return to position' }).click();

    expect(onClick).toHaveBeenCalledOnce();
  });
});
