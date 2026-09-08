import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import DataLibrary from './DataLibrary.svelte';

it('groups and sorts sources without changing the input order', async () => {
  let airspace = {
    generation: 1,
    sources: [
      { type: 'disabled' as const, sourceName: 'Zulu.txt' },
      { type: 'active' as const, sourceName: 'Alpha.txt', airspaceCount: 1 },
    ],
  };
  let component = await render(DataLibrary, {
    airspace,
    waypoints: {
      generation: 2,
      sources: [
        {
          type: 'active',
          sourceName: 'Alpha.txt',
          waypointCount: 2,
          warnings: [{ line: 4, message: 'Skipped waypoint' }],
        },
        { type: 'unavailable', sourceName: 'broken.cup', error: 'parseFailed' },
      ],
    },
  });
  let groups = [...document.querySelectorAll('section')];
  expect(groups.map((group) => group.querySelector('h2')?.textContent)).toEqual([
    'Airspace',
    'Waypoints',
  ]);
  expect([...groups[0].querySelectorAll('h3')].map((name) => name.textContent)).toEqual([
    'Alpha.txt',
    'Zulu.txt',
  ]);
  expect(airspace.sources.map((source) => source.sourceName)).toEqual(['Zulu.txt', 'Alpha.txt']);
  await expect.element(page.getByText('Imported · 1 airspace', { exact: true })).toBeVisible();
  await expect
    .element(page.getByText('Imported · 2 waypoints · 1 warning', { exact: true }))
    .toBeVisible();
  await expect.element(page.getByText('Disabled', { exact: true })).toBeVisible();
  await expect
    .element(page.getByText('Imported · could not be parsed', { exact: true }))
    .toBeVisible();
  await component.rerender({
    airspace: { generation: 2, sources: [] },
    waypoints: { generation: 3, sources: [] },
  });
  await expect.element(page.getByText('No data on this device', { exact: true })).toBeVisible();
  expect(document.querySelectorAll('section')).toHaveLength(0);
  let icon = document.querySelector('.i-mdi-database-outline')!;
  expect(getComputedStyle(icon).display).toBe('inline-block');
});

it.each([413, 544, 915])('keeps rows inside the responsive card at width %s', async (width) => {
  let previous = { width: window.innerWidth, height: window.innerHeight };
  try {
    await page.viewport(width, 600);
    await render(DataLibrary, {
      airspace: { generation: 0, sources: [] },
      waypoints: {
        generation: 1,
        sources: [
          {
            type: 'disabled',
            sourceName:
              'A very long imported waypoint filename that must wrap without horizontal overflow.cup',
          },
        ],
      },
    });
    let row = page.getByRole('listitem').element();
    let bounds = row.getBoundingClientRect();
    expect([bounds.left, bounds.right]).toEqual(
      width <= 544 ? [0, width] : [(width - 544) / 2 + 20, (width + 544) / 2 - 20],
    );
    expect(row.scrollWidth).toBe(row.clientWidth);
    await expect
      .element(page.getByRole('link', { name: 'Back to settings' }))
      .toHaveAttribute('href', '/settings');
  } finally {
    await page.viewport(previous.width, previous.height);
  }
});
