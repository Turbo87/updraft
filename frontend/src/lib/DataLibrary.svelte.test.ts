import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import '../app.css';

import { FakeClient } from './client/fake';
import DataLibrary from './DataLibrary.svelte';
import { AirspaceStore } from './stores/airspace.svelte';
import { DataActivation } from './stores/data-activation.svelte';
import { WaypointsStore } from './stores/waypoints.svelte';

function activation() {
  return new DataActivation(
    { setAirspaceEnabled: vi.fn(), setWaypointsEnabled: vi.fn() },
    new AirspaceStore(),
    new WaypointsStore(),
  );
}

it('shows the latest activation choice without disabling the control', async () => {
  let airspace = new AirspaceStore();
  let waypoints = new WaypointsStore();
  airspace.current = {
    generation: 1,
    sources: [{ type: 'unavailable', sourceName: 'broken.txt', error: 'parseFailed' }],
  };
  let result = Promise.withResolvers<void>();
  let client = {
    setAirspaceEnabled: vi.fn().mockReturnValue(result.promise),
    setWaypointsEnabled: vi.fn(),
  };
  let changes = new DataActivation(client, airspace, waypoints);
  await render(DataLibrary, {
    importer: new FakeClient(),
    airspace: airspace.current,
    waypoints: waypoints.current,
    activation: changes,
    onRemove: vi.fn(),
  });
  await page.getByRole('button', { name: /^broken.txt/ }).click();
  let toggle = page.getByRole('switch', { name: 'Enabled', exact: true });
  await expect.element(toggle).toBeChecked();
  await userEvent.keyboard('{Tab}');
  await expect.element(toggle).toHaveFocus();
  let input = toggle.element();
  let visual = input.nextElementSibling!;
  expect(getComputedStyle(input).opacity).toBe('0');
  expect([visual.getBoundingClientRect().width, visual.getBoundingClientRect().height]).toEqual([
    28, 28,
  ]);
  expect(getComputedStyle(visual).outlineStyle).toBe('solid');
  await toggle.click();
  await expect.element(toggle).not.toBeChecked();
  await expect.element(toggle).toBeEnabled();
  await expect.element(page.getByText('Imported · could not be parsed')).not.toBeInTheDocument();
  await expect.element(page.getByRole('button', { name: 'Remove from device' })).toBeDisabled();
  result.reject(new Error('storage failed'));
  await expect.element(toggle).toBeChecked();
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not change file activation.');
  await userEvent.keyboard('{Escape}');
});

it('opens live file details and confirms removal separately', async () => {
  let onRemove = vi
    .fn()
    .mockRejectedValueOnce(new Error('storage failed'))
    .mockResolvedValue(undefined);
  let airspace = {
    generation: 1,
    sources: [{ type: 'disabled' as const, sourceName: 'local.txt' }],
  };
  let waypoints = {
    generation: 1,
    sources: [
      {
        type: 'active' as const,
        sourceName: 'local.txt',
        waypointCount: 2,
        warnings: [{ line: 4, message: 'Skipped waypoint' }],
      },
    ],
  };
  let view = await render(DataLibrary, {
    importer: new FakeClient(),
    airspace,
    waypoints,
    onRemove,
    activation: activation(),
  });
  let row = page.getByRole('region', { name: 'Waypoints' }).getByRole('button');
  await row.click();
  let dialog = page.getByRole('dialog', { name: 'local.txt' });
  await expect.element(dialog).toBeVisible();
  await expect.element(dialog.getByText('Line 4: Skipped waypoint')).toBeVisible();
  await view.rerender({
    waypoints: {
      generation: 2,
      sources: [{ type: 'unavailable', sourceName: 'local.txt', error: 'parseFailed' }],
    },
  });
  await expect.element(dialog.getByText('Imported · could not be parsed')).toBeVisible();
  await expect.element(dialog.getByText('Line 4: Skipped waypoint')).not.toBeInTheDocument();
  await view.rerender({ waypoints });
  await userEvent.keyboard('{Escape}');
  await expect.element(dialog).not.toBeInTheDocument();
  await expect.element(row).toHaveFocus();
  await row.click();
  await page.getByRole('button', { name: 'Close', exact: true }).click();
  await expect.element(dialog).not.toBeInTheDocument();
  await row.click();
  await page.getByRole('button', { name: 'Remove from device' }).click();
  await expect.element(dialog).not.toBeInTheDocument();
  await expect.element(page.getByRole('alertdialog')).toBeVisible();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect.element(dialog).not.toBeInTheDocument();
  expect(onRemove).not.toHaveBeenCalled();
  await row.click();
  await page.getByRole('button', { name: 'Remove from device' }).click();
  await page.getByRole('button', { name: 'Remove', exact: true }).click();
  await expect.element(page.getByRole('alert')).toHaveTextContent('Could not remove the file.');
  await page.getByRole('button', { name: 'Remove', exact: true }).click();
  expect(onRemove.mock.calls).toEqual([
    ['waypoints', 'local.txt'],
    ['waypoints', 'local.txt'],
  ]);
  await expect.element(page.getByRole('alertdialog')).not.toBeInTheDocument();
  await view.rerender({ airspace, waypoints: { generation: 2, sources: [] } });
  await page.getByRole('region', { name: 'Airspace' }).getByRole('button').click();
  await expect.element(dialog.getByRole('switch', { name: 'Enabled' })).not.toBeChecked();
  await view.rerender({ airspace: { generation: 2, sources: [] } });
  await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
});

it('groups and sorts sources without changing the input order', async () => {
  let airspace = {
    generation: 1,
    sources: [
      { type: 'disabled' as const, sourceName: 'Zulu.txt' },
      { type: 'active' as const, sourceName: 'Alpha.txt', airspaceCount: 1 },
    ],
  };
  let component = await render(DataLibrary, {
    importer: new FakeClient(),
    activation: activation(),
    onRemove: vi.fn(),
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
  expect([...groups[0].querySelectorAll('.filename')].map((name) => name.textContent)).toEqual([
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
      importer: new FakeClient(),
      activation: activation(),
      onRemove: vi.fn(),
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

it('confirms a same-name replacement and discards cancellation', async () => {
  let selected = { selectionId: '1', sourceName: 'local.cup', dataType: 'waypoints' as const };
  let importer = {
    selectDataFile: vi.fn().mockResolvedValue(selected),
    importDataFile: vi.fn().mockResolvedValue(selected),
    discardDataFile: vi.fn().mockResolvedValue(undefined),
  };
  await render(DataLibrary, {
    airspace: { generation: 0, sources: [] },
    waypoints: { generation: 1, sources: [{ type: 'disabled', sourceName: 'local.cup' }] },
    activation: activation(),
    onRemove: vi.fn(),
    importer,
  });
  await page.getByRole('button', { name: 'Add data' }).click();
  await expect.element(page.getByRole('alertdialog')).toBeVisible();
  expect(importer.importDataFile).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect.poll(() => importer.discardDataFile.mock.calls).toEqual([['1']]);
  await expect.element(page.getByRole('button', { name: 'Add data' })).toHaveFocus();
  await page.getByRole('button', { name: 'Add data' }).click();
  await page.getByRole('button', { name: 'Replace file', exact: true }).click();
  await expect.poll(() => importer.importDataFile.mock.calls).toEqual([['1']]);
  await expect.element(page.getByRole('alertdialog')).not.toBeInTheDocument();
  await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
});

it('imports a new dataset without confusing filenames in another group', async () => {
  let selected = { selectionId: '2', sourceName: 'local.cup', dataType: 'waypoints' as const };
  let importer = {
    selectDataFile: vi.fn().mockResolvedValueOnce(null).mockResolvedValue(selected),
    importDataFile: vi
      .fn()
      .mockRejectedValueOnce(new Error('storage failed'))
      .mockResolvedValue(selected),
    discardDataFile: vi.fn(),
  };
  await render(DataLibrary, {
    importer,
    activation: activation(),
    onRemove: vi.fn(),
    airspace: { generation: 1, sources: [{ type: 'disabled', sourceName: 'local.cup' }] },
    waypoints: { generation: 0, sources: [] },
  });
  let add = page.getByRole('button', { name: 'Add data' });
  await add.click();
  await expect.element(add).toBeEnabled();
  expect(importer.importDataFile).not.toHaveBeenCalled();
  await add.click();
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not import the file. Select it again to retry.');
  await expect.element(page.getByRole('alertdialog')).not.toBeInTheDocument();
  await add.click();
  await expect.poll(() => importer.importDataFile.mock.calls).toEqual([['2'], ['2']]);
  await expect.element(page.getByRole('alert')).not.toBeInTheDocument();
});

it('discards a picker result when the library has been closed', async () => {
  let result = Promise.withResolvers<null | {
    selectionId: string;
    sourceName: string;
    dataType: 'airspace';
  }>();
  let importer = {
    selectDataFile: vi.fn().mockReturnValue(result.promise),
    importDataFile: vi.fn(),
    discardDataFile: vi.fn().mockResolvedValue(undefined),
  };
  let view = await render(DataLibrary, {
    importer,
    activation: activation(),
    onRemove: vi.fn(),
    airspace: { generation: 0, sources: [] },
    waypoints: { generation: 0, sources: [] },
  });
  await page.getByRole('button', { name: 'Add data' }).click();
  await view.unmount();
  result.resolve({ selectionId: '3', sourceName: 'local.txt', dataType: 'airspace' });
  await expect.poll(() => importer.discardDataFile.mock.calls).toEqual([['3']]);
  expect(importer.importDataFile).not.toHaveBeenCalled();
});

it('moves Add data from the footer to the header above 544px', async () => {
  let previous = { width: window.innerWidth, height: window.innerHeight };
  await render(DataLibrary, {
    importer: new FakeClient(),
    activation: activation(),
    onRemove: vi.fn(),
    airspace: { generation: 0, sources: [] },
    waypoints: { generation: 0, sources: [] },
  });
  try {
    for (let width of [413, 544, 545, 915]) {
      await page.viewport(width, 600);
      let add = page.getByRole('button', { name: 'Add data' });
      await expect.element(add).toBeVisible();
      expect(add.element().closest(width > 544 ? 'header' : 'footer')).not.toBeNull();
      expect(add.element().getBoundingClientRect().height).toBe(width > 544 ? 48 : 56);
    }
  } finally {
    await page.viewport(previous.width, previous.height);
  }
});

it.each([
  { position: 'new below', statusFirst: false },
  { position: 'new below', statusFirst: true },
  { position: 'replacement below', statusFirst: false },
  { position: 'replacement above', statusFirst: true },
  { position: 'visible', statusFirst: false },
  { position: 'new below', statusFirst: true, dataType: 'airspace' as const },
  { position: 'replacement below', statusFirst: true, fails: true },
])(
  'reveals $position after import with statusFirst=$statusFirst',
  async ({ position, statusFirst, dataType = 'waypoints', fails = false }) => {
    let extension = dataType === 'airspace' ? 'txt' : 'cup';
    let sources = Array.from({ length: 30 }, (_, i) => ({
      type: 'disabled' as const,
      sourceName: `file${String(i).padStart(2, '0')}.${extension}`,
    }));
    let sourceName =
      position === 'new below'
        ? `file30.${extension}`
        : position === 'replacement below'
          ? `file29.${extension}`
          : `file00.${extension}`;
    let selection = { selectionId: '1', dataType, sourceName };
    let completion = Promise.withResolvers<typeof selection>();
    let importer = {
      selectDataFile: vi.fn().mockResolvedValue(selection),
      importDataFile: vi.fn().mockReturnValue(completion.promise),
      discardDataFile: vi.fn(),
    };
    let view = await render(DataLibrary, {
      importer,
      activation: activation(),
      onRemove: vi.fn(),
      airspace: { generation: 0, sources: [] },
      waypoints: { generation: 0, sources: [] },
      [dataType]: { generation: 1, sources },
    });
    let main = page.getByRole('main').element();
    main.parentElement!.style.height = '320px';
    if (position === 'replacement above') main.scrollTop = main.scrollHeight;
    let initialScroll = main.scrollTop;
    await page.getByRole('button', { name: 'Add data' }).click();
    if (position !== 'new below')
      await page.getByRole('button', { name: 'Replace file', exact: true }).click();
    await expect.poll(() => importer.importDataFile.mock.calls.length).toBe(1);
    let updated = {
      generation: 2,
      sources: [
        ...sources.filter((source) => source.sourceName !== sourceName),
        { type: 'unavailable' as const, sourceName, error: 'parseFailed' as const },
      ],
    };
    if (statusFirst) {
      await view.rerender({ [dataType]: updated });
      expect(main.scrollTop).toBe(initialScroll);
      if (fails) completion.reject(new Error('publication failed'));
      else completion.resolve(selection);
    } else {
      completion.resolve(selection);
      await expect.element(page.getByRole('button', { name: 'Add data' })).toBeEnabled();
      expect(main.scrollTop).toBe(initialScroll);
      await view.rerender({ [dataType]: updated });
    }
    let row = page.getByRole('button', {
      name: `${sourceName} Imported · could not be parsed`,
      exact: true,
    });
    await expect.element(row).toBeVisible();
    if (position === 'visible' || fails) expect(main.scrollTop).toBe(initialScroll);
    else
      await expect
        .poll(() => {
          let edge: 'top' | 'bottom' = position === 'replacement above' ? 'top' : 'bottom';
          return Math.abs(
            Math.round(
              row.element().getBoundingClientRect()[edge] - main.getBoundingClientRect()[edge],
            ),
          );
        })
        .toBe(0);
    let finalScroll = main.scrollTop;
    main.scrollTop = 0;
    await view.rerender({ [dataType]: { ...updated, generation: 3 } });
    expect(main.scrollTop).toBe(0);
    if (position.includes('below') && !fails) expect(finalScroll).toBeGreaterThan(initialScroll);
    await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
  },
);
