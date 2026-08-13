import { createRawSnippet } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import Button from './Button.svelte';

const children = createRawSnippet(() => ({ render: () => '<span>Save changes</span>' }));

describe('Button.svelte', () => {
  it('renders a primary standard button and forwards clicks', async () => {
    let onclick = vi.fn();
    render(Button, { children, onclick });

    let button = page.getByRole('button', { name: 'Save changes' });
    await button.click();

    expect(onclick).toHaveBeenCalledOnce();
    await expect.element(button).toHaveAttribute('type', 'button');
    await expect.element(button).toHaveClass(/primary/);
    await expect.element(button).toHaveClass(/standard/);
  });

  it('uses the standard and large control heights', async () => {
    let view = await render(Button, { children });
    let button = page.getByRole('button', { name: 'Save changes' });

    expect(getComputedStyle(button.element()).height).toBe('48px');

    await view.rerender({ children, size: 'large' });

    expect(getComputedStyle(button.element()).height).toBe('56px');
  });

  it('disables the button and reports progress while loading', async () => {
    render(Button, { children, loading: true });

    let button = page.getByRole('button', { name: 'Save changes' });
    await expect.element(button).toBeDisabled();
    await expect.element(button).toHaveAttribute('aria-busy', 'true');
    expect(button.element().querySelector('.i-mdi-loading')).not.toBeNull();
  });

  it.each([
    ['primary', '--color-action-primary-surface', '--color-action-primary-text'],
    ['secondary', '--color-action-secondary-surface', '--color-action-secondary-text'],
    ['destructive', '--color-action-destructive-surface', '--color-action-destructive-text'],
  ] as const)('fades the %s colors when disabled', (variant, surface, text) => {
    render(Button, { children, disabled: true, variant });
    let button = page.getByRole('button', { name: 'Save changes' }).element();
    let reference = document.createElement('div');
    reference.style.background = `var(${surface})`;
    reference.style.color = `var(${text})`;
    document.body.append(reference);

    let buttonStyle = getComputedStyle(button);
    let referenceStyle = getComputedStyle(reference);
    let presentation = [buttonStyle.backgroundColor, buttonStyle.color, buttonStyle.opacity];
    let expectedPresentation = [referenceStyle.backgroundColor, referenceStyle.color, '0.45'];

    reference.remove();

    expect(presentation).toEqual(expectedPresentation);
  });

  it('uses a visible dark secondary pressed surface', () => {
    let root = document.documentElement;
    let previousTheme = root.dataset.theme;
    root.dataset.theme = 'dark';

    render(Button, { children, variant: 'secondary' });
    let button = page.getByRole('button', { name: 'Save changes' }).element();
    let restSurface = getComputedStyle(button).backgroundColor;

    button.style.transition = 'none';
    button.style.background = 'var(--button-pressed-surface)';
    let pressedSurface = getComputedStyle(button).backgroundColor;

    if (previousTheme) root.dataset.theme = previousTheme;
    else delete root.dataset.theme;

    expect(pressedSurface).not.toBe(restSurface);
  });
});
