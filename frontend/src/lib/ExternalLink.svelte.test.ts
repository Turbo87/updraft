import { createRawSnippet } from 'svelte';
import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import ExternalLink from './ExternalLink.svelte';

const tauri = vi.hoisted(() => ({
  isTauri: vi.fn(() => true),
  openUrl: vi.fn(() => Promise.resolve()),
}));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: tauri.isTauri }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: tauri.openUrl }));

const children = createRawSnippet(() => ({ render: () => '<span>GitHub repository</span>' }));

it('keeps native link semantics and forwards anchor attributes', async () => {
  let onClick = vi.fn();
  render(ExternalLink, {
    children,
    class: 'credits-link',
    href: 'https://github.com/Turbo87/updraft',
    hreflang: 'en',
    onclick: onClick,
    referrerpolicy: 'no-referrer',
  });

  let link = page.getByRole('link', { name: 'GitHub repository' });
  await expect.element(link).toHaveAttribute('href', 'https://github.com/Turbo87/updraft');
  await expect.element(link).toHaveAttribute('class', 'credits-link');
  await expect.element(link).toHaveAttribute('hreflang', 'en');
  await expect.element(link).toHaveAttribute('referrerpolicy', 'no-referrer');
  await expect.element(link).toHaveAttribute('target', '_blank');
  await expect.element(link).toHaveAttribute('rel', 'noopener noreferrer');
  expect(link.element().querySelector('.i-mdi-open-in-new')).toBeNull();

  await link.click();

  expect(onClick).toHaveBeenCalledOnce();
  expect(tauri.openUrl).toHaveBeenCalledExactlyOnceWith('https://github.com/Turbo87/updraft');
});
