import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import '../app.css';

import IconButton from './IconButton.svelte';

const props = { icon: 'i-mdi-download', label: 'Update', style: 'transition: none' };

describe('IconButton.svelte', () => {
  it('renders a named round target and forwards clicks', async () => {
    let onclick = vi.fn();
    render(IconButton, { ...props, onclick });
    let button = page.getByRole('button', { name: 'Update' });
    await button.click();
    expect(onclick).toHaveBeenCalledOnce();
    await expect.element(button).toHaveAttribute('type', 'button');
    let style = getComputedStyle(button.element());
    expect([style.width, style.height, style.borderRadius]).toEqual(['48px', '48px', '50%']);
    expect(button.element().querySelector('.i-mdi-download')?.getAttribute('aria-hidden')).toBe(
      'true',
    );
  });

  it('prevents activation when disabled', async () => {
    let onclick = vi.fn();
    render(IconButton, { ...props, disabled: true, onclick });
    let button = page.getByRole('button', { name: 'Update' });
    await expect.element(button).toBeDisabled();
    (button.element() as HTMLButtonElement).click();
    expect(onclick).not.toHaveBeenCalled();
  });

  it.each(['light', 'dark'] as const)(
    'uses raised resting and pressed surfaces in %s mode',
    async (theme) => {
      let root = document.documentElement;
      let previousTheme = root.dataset.theme;
      root.dataset.theme = theme;
      let reference = document.createElement('div');
      document.body.append(reference);
      try {
        await render(IconButton, props);
        let button = page.getByRole('button', { name: 'Update' }).element();
        reference.style.background = `var(--color-slate-${theme === 'light' ? '100' : '700'})`;
        expect(getComputedStyle(button).backgroundColor).toBe(
          getComputedStyle(reference).backgroundColor,
        );
        (button as HTMLButtonElement).focus();
        await userEvent.keyboard('[Space>]');
        try {
          reference.style.background = `var(--color-slate-${theme === 'light' ? '200' : '600'})`;
          expect(getComputedStyle(button).backgroundColor).toBe(
            getComputedStyle(reference).backgroundColor,
          );
          expect(getComputedStyle(button).outlineStyle).toBe('solid');
          expect(getComputedStyle(button).outlineWidth).toBe('2px');
        } finally {
          await userEvent.keyboard('[/Space]');
        }
      } finally {
        reference.remove();
        if (previousTheme === undefined) delete root.dataset.theme;
        else root.dataset.theme = previousTheme;
      }
    },
  );
});
