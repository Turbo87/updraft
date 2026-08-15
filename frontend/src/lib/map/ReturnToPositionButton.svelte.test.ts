import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../../app.css';

import ReturnToPositionButton from './ReturnToPositionButton.svelte';

it('uses the MDI icon without a global stacking override', async () => {
  render(ReturnToPositionButton, { onClick: () => {} });

  let button = page.getByRole('button', { name: 'Return to position' });
  await expect.element(button).toBeVisible();

  let buttonElement = document.querySelector<HTMLButtonElement>(
    'button[aria-label="Return to position"]',
  );
  if (!buttonElement) throw new Error('Return to position button is not available');

  let icon = buttonElement.querySelector<HTMLElement>('.i-mdi-crosshairs-gps');
  if (!icon) throw new Error('Return to position icon is not available');

  expect(icon.getBoundingClientRect().width).toBe(28);
  expect(icon.getBoundingClientRect().height).toBe(28);
  expect(buttonElement.getBoundingClientRect().width).toBe(56);
  expect(buttonElement.getBoundingClientRect().height).toBe(56);
  expect(getComputedStyle(buttonElement).zIndex).toBe('auto');
});
