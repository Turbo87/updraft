import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import AboutScreen from './AboutScreen.svelte';

const BUILD_TIMESTAMP = '2026-08-12T07:14:22.000Z';
const FULL_COMMIT_SHA = 'a1c93f456789abcdef0123456789abcdef012345';

it('shows build details, source links, credits, and licence information', async () => {
  render(AboutScreen, {
    attributions: [
      'Base map tiles by <a href="https://openfreemap.org">OpenFreeMap</a>, data © <a href="https://www.openstreetmap.org/copyright">OpenStreetMap contributors</a>.',
    ],
    commitSha: FULL_COMMIT_SHA,
    locale: 'en',
    timestamp: BUILD_TIMESTAMP,
  });

  await expect.element(page.getByRole('heading', { name: 'About' })).toBeInTheDocument();
  await expect
    .element(page.getByRole('link', { name: 'a1c93f4' }))
    .toHaveAttribute('href', `https://github.com/Turbo87/updraft/commit/${FULL_COMMIT_SHA}`);
  await expect
    .element(page.getByRole('link', { name: 'GitHub repository' }))
    .toHaveAttribute('href', 'https://github.com/Turbo87/updraft');
  await expect
    .element(page.getByRole('link', { name: 'OpenFreeMap' }))
    .toHaveAttribute('href', 'https://openfreemap.org/');
  for (let linkName of ['a1c93f4', 'GitHub repository', 'OpenFreeMap']) {
    let link = page.getByRole('link', { name: linkName });
    await expect.element(link).toHaveAttribute('target', '_blank');
    await expect.element(link).toHaveAttribute('rel', 'noopener noreferrer');
  }
  await expect.element(page.getByRole('heading', { name: 'Licences' })).toBeInTheDocument();
});

it('shows an unknown version and omits empty data credits', async () => {
  render(AboutScreen, {
    attributions: [],
    commitSha: undefined,
    locale: 'en',
    timestamp: BUILD_TIMESTAMP,
  });

  await expect.element(page.getByText('Unknown version')).toBeInTheDocument();
  await expect.element(page.getByRole('heading', { name: 'Data credits' })).not.toBeInTheDocument();
});
