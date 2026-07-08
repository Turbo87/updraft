import { page, userEvent } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import MapDebugOverlay from './MapDebugOverlay.svelte';

describe('MapDebugOverlay.svelte', () => {
  it('is hidden until the D key is pressed, then toggles off again', async () => {
    render(MapDebugOverlay, { map: undefined });

    let zoom = page.getByText('Zoom');
    await expect.element(zoom).not.toBeInTheDocument();

    await userEvent.keyboard('d');
    await expect.element(zoom).toBeInTheDocument();

    await userEvent.keyboard('d');
    await expect.element(zoom).not.toBeInTheDocument();
  });

  it('offers a tile-boundaries checkbox once visible', async () => {
    render(MapDebugOverlay, { map: undefined });

    await userEvent.keyboard('d');

    let checkbox = page.getByRole('checkbox', { name: 'Tile boundaries' });
    await expect.element(checkbox).not.toBeChecked();
    await checkbox.click();
    await expect.element(checkbox).toBeChecked();
  });
});
