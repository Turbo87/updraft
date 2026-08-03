import type { Map, Source } from 'maplibre-gl';

import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import DataCredits from './DataCredits.svelte';
import { collectMapSourceAttributions } from './map-attribution';

it('collects unique source attributions in style order', () => {
  let attributions = {
    first: ' First credit ',
    duplicate: 'First credit',
    empty: '  ',
    second: 'Second credit',
  };
  let map = {
    getStyle: () => ({
      sources: Object.fromEntries(Object.keys(attributions).map((id) => [id, {}])),
    }),
    getSource: (id: keyof typeof attributions) => ({ attribution: attributions[id] }) as Source,
  } as unknown as Map;

  expect(collectMapSourceAttributions(map)).toEqual(['First credit', 'Second credit']);
});

it('renders safe attribution text and links without active content', async () => {
  let safeAttribution =
    '<a href="https://example.com/data">Open Data</a> and <a href="http://example.com/terrain"><strong>Terrain</strong></a>';
  render(DataCredits, {
    attributions: [
      safeAttribution,
      'Unsafe <a href="javascript:alert(1)">link</a><script>hidden script</script><img alt="hidden image">',
      '   ',
      ` ${safeAttribution} `,
    ],
  });

  await expect.element(page.getByRole('heading', { name: 'Data credits' })).toBeInTheDocument();
  expect(document.querySelectorAll('li')).toHaveLength(2);
  let list = document.querySelector('ul');
  if (!list) throw new Error('Data credits list is not available');
  expect(getComputedStyle(list).listStyleType).toBe('disc');
  await expect
    .element(page.getByRole('link', { name: 'Open Data' }))
    .toHaveAttribute('href', 'https://example.com/data');
  await expect
    .element(page.getByRole('link', { name: 'Terrain' }))
    .toHaveAttribute('href', 'http://example.com/terrain');
  await expect.element(page.getByText('Unsafe link')).toBeInTheDocument();
  await expect.element(page.getByRole('link', { name: 'link' })).not.toBeInTheDocument();
  await expect.element(page.getByText('hidden script')).not.toBeInTheDocument();
  await expect.element(page.getByRole('img', { name: 'hidden image' })).not.toBeInTheDocument();
});

it('omits the section when no attribution has visible text', async () => {
  render(DataCredits, { attributions: ['', ' ', '<script>hidden</script>', '<img alt="hidden">'] });

  await expect.element(page.getByRole('heading', { name: 'Data credits' })).not.toBeInTheDocument();
});
