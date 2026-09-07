import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

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

it.each([413, 544, 915])('lays out About cards at viewport width %s', async (width) => {
  let oldWidth = window.innerWidth;
  let oldHeight = window.innerHeight;
  let root = document.documentElement;
  let previousStyle = root.getAttribute('style');
  try {
    await page.viewport(width, 600);
    root.style.setProperty('--safe-area-left', '24px');
    root.style.setProperty('--safe-area-right', '12px');
    render(AboutScreen, {
      attributions: [
        'Map data © <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>.',
      ],
      commitSha: FULL_COMMIT_SHA,
      locale: 'en',
      timestamp: BUILD_TIMESTAMP,
    });
    for (let name of ['GitHub repository', 'OpenStreetMap']) {
      let link = page.getByRole('link', { name }).element() as HTMLAnchorElement;
      let bounds = link.getBoundingClientRect();
      let sectionBounds = link.closest('section')!.getBoundingClientRect();
      expect([bounds.left, bounds.right]).toEqual(
        width <= 544 ? [0, width] : [sectionBounds.left, sectionBounds.right],
      );
      expect([getComputedStyle(link).paddingLeft, getComputedStyle(link).paddingRight]).toEqual(
        width <= 544 ? ['44px', '28px'] : ['20px', '16px'],
      );
      expect(getComputedStyle(link).borderTopWidth).toBe('1px');
      link.focus();
      expect(link.matches(':focus-visible')).toBe(true);
      expect(getComputedStyle(link).outlineOffset).toBe('-2px');
    }
    let licenceSection = page
      .getByRole('heading', { name: 'Licences' })
      .element()
      .closest('section')!;
    let licence = licenceSection.querySelector('p')!;
    let bounds = licence.getBoundingClientRect();
    let sectionBounds = licenceSection.getBoundingClientRect();
    expect([bounds.left, bounds.right]).toEqual(
      width <= 544 ? [0, width] : [sectionBounds.left, sectionBounds.right],
    );
    expect(getComputedStyle(licence).paddingLeft).toBe(width <= 544 ? '44px' : '20px');
    expect(getComputedStyle(licence).paddingRight).toBe(width <= 544 ? '32px' : '20px');
  } finally {
    if (previousStyle === null) root.removeAttribute('style');
    else root.setAttribute('style', previousStyle);
    await page.viewport(oldWidth, oldHeight);
  }
});
