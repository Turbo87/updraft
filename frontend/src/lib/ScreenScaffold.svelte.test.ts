import { createRawSnippet } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import ScreenScaffold from './ScreenScaffold.svelte';

const children = createRawSnippet(() => ({ render: () => '<p>Screen content</p>' }));
const actions = createRawSnippet(() => ({ render: () => '<button>Save changes</button>' }));

describe('ScreenScaffold.svelte', () => {
  it('renders a link return control and a scrolling content region', async () => {
    render(ScreenScaffold, {
      backHref: '/settings',
      backLabel: 'Back to settings',
      children,
      title: 'Units',
    });

    let back = page.getByRole('link', { name: 'Back to settings' });
    let content = page.getByRole('main');

    await expect.element(back).toHaveAttribute('href', '/settings');
    await expect.element(page.getByRole('heading', { level: 1, name: 'Units' })).toBeVisible();
    await expect.element(content).toHaveAttribute('tabindex', '0');
    expect(getComputedStyle(back.element()).height).toBe('56px');
    expect(getComputedStyle(content.element()).overflowY).toBe('auto');
  });

  it('renders a callback return control', async () => {
    let onBack = vi.fn();
    render(ScreenScaffold, {
      backLabel: 'Back',
      children,
      onBack,
      title: 'Traffic details',
    });

    let back = page.getByRole('button', { name: 'Back' });
    await back.click();

    expect(onBack).toHaveBeenCalledOnce();
    await expect.element(back).toHaveAttribute('type', 'button');
  });

  it('truncates a long title without horizontal overflow', () => {
    render(ScreenScaffold, {
      backLabel: 'Back',
      children,
      onBack: () => {},
      title: 'LF-R196A1 TEST GAP (NOTAM) '.repeat(10),
    });

    let scaffold = page.getByRole('heading', { level: 1 }).element().closest('.screen-scaffold');
    expect(scaffold).not.toBeNull();
    expect(scaffold!.scrollWidth).toBe(scaffold!.clientWidth);
  });

  it('renders an optional fixed action bar outside the scrolling region', async () => {
    render(ScreenScaffold, {
      actions,
      backHref: '/settings',
      backLabel: 'Back to settings',
      children,
      title: 'Units',
    });

    let content = page.getByRole('main').element();
    let actionBar = page.getByRole('contentinfo').element();

    await expect.element(page.getByRole('button', { name: 'Save changes' })).toBeVisible();
    expect(actionBar.parentElement).toBe(content.parentElement);
    expect(content.contains(actionBar)).toBe(false);
  });
});
