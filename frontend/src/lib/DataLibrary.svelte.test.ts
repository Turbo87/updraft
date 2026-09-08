import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import '../app.css';

import { FakeClient } from './client/fake';
import DataLibrary from './DataLibrary.svelte';
import { AirspaceStore } from './stores/airspace.svelte';
import { BasemapsStore } from './stores/basemaps.svelte';
import { DataActivation } from './stores/data-activation.svelte';
import { TerrainStore } from './stores/terrain.svelte';
import { WaypointsStore } from './stores/waypoints.svelte';

function activation() {
  return new DataActivation(
    {
      setAirspaceEnabled: vi.fn(),
      setTerrainEnabled: vi.fn(),
      setBasemapEnabled: vi.fn(),
      setWaypointsEnabled: vi.fn(),
    },
    new AirspaceStore(),
    new WaypointsStore(),
    new BasemapsStore(),
    new TerrainStore(),
  );
}

it.each([
  ['basemap', 'mbtiles', 'basemaps'],
  ['terrain', 'terrain', 'terrain'],
] as const)(
  'shows managed %s filenames and removes the selected full identity',
  async (type, extension, prop) => {
    let filename = `Germany.${extension}`;
    let sourceName = `enroute/Europe/${filename}`;
    let onRemove = vi.fn().mockResolvedValue(undefined);
    await render(DataLibrary, {
      catalog: null,
      onRetryCatalog: vi.fn(),
      onCheckBasemapUpdates: vi.fn(async () => []),
      onDownload: vi.fn(),
      onCancelDownload: vi.fn(),
      importer: new FakeClient(),
      activation: activation(),
      onRemove,
      airspace: { generation: 0, sources: [] },
      waypoints: { generation: 0, sources: [] },
      [prop]: { generation: 0, sources: [{ sourceName, type: 'active' }] },
    });
    await page.getByRole('button', { name: new RegExp(`^Germany\\.${extension}`) }).click();
    await expect.element(page.getByRole('heading', { name: filename, exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Remove from device', exact: true }).click();
    await page
      .getByRole('alertdialog')
      .getByRole('button', { name: 'Remove', exact: true })
      .click();
    expect(onRemove).toHaveBeenCalledWith(type, sourceName);
  },
);

it.each(['airspace', 'basemap', 'terrain'] as const)(
  'shows the latest %s choice without disabling the control',
  async (type) => {
    let airspace = new AirspaceStore();
    let waypoints = new WaypointsStore();
    let basemaps = new BasemapsStore();
    let terrain = new TerrainStore();
    if (type === 'airspace') {
      airspace.current = {
        generation: 1,
        sources: [{ type: 'unavailable', sourceName: 'broken.txt', error: 'parseFailed' }],
      };
    } else {
      let store = type === 'basemap' ? basemaps : terrain;
      store.current = {
        generation: 1,
        sources: [
          {
            type: 'unavailable',
            sourceName: type === 'basemap' ? 'broken.mbtiles' : 'broken.terrain',
          },
        ],
      };
    }
    let result = Promise.withResolvers<void>();
    let client = {
      setAirspaceEnabled: vi.fn().mockReturnValue(result.promise),
      setBasemapEnabled: vi.fn().mockReturnValue(result.promise),
      setTerrainEnabled: vi.fn().mockReturnValue(result.promise),
      setWaypointsEnabled: vi.fn(),
    };
    let changes = new DataActivation(client, airspace, waypoints, basemaps, terrain);
    await render(DataLibrary, {
      catalog: null,
      onRetryCatalog: vi.fn(),
      onCheckBasemapUpdates: vi.fn(async () => []),
      onDownload: vi.fn(),
      onCancelDownload: vi.fn(),
      importer: new FakeClient(),
      basemaps: basemaps.current,
      terrain: terrain.current,
      airspace: airspace.current,
      waypoints: waypoints.current,
      activation: changes,
      onRemove: vi.fn(),
    });
    await page.getByRole('button', { name: /^broken\./ }).click();
    let toggle = page.getByRole('switch', { name: 'Enabled', exact: true });
    await expect.element(toggle).toBeChecked();
    await expect.element(page.getByRole('button', { name: 'Close', exact: true })).toHaveFocus();
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
    let loadError =
      type === 'airspace' ? 'Imported · could not be parsed' : 'Could not load the file.';
    await expect.element(page.getByText(loadError)).not.toBeInTheDocument();
    await expect.element(page.getByRole('button', { name: 'Remove from device' })).toBeDisabled();
    result.reject(new Error('storage failed'));
    await expect.element(toggle).toBeChecked();
    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Could not change file activation.');
    await userEvent.keyboard('{Escape}');
  },
);

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
    catalog: null,
    onRetryCatalog: vi.fn(),
    onCheckBasemapUpdates: vi.fn(async () => []),
    onDownload: vi.fn(),
    onCancelDownload: vi.fn(),
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
    catalog: null,
    onRetryCatalog: vi.fn(),
    onCheckBasemapUpdates: vi.fn(async () => []),
    onDownload: vi.fn(),
    onCancelDownload: vi.fn(),
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
      catalog: null,
      onRetryCatalog: vi.fn(),
      onCheckBasemapUpdates: vi.fn(async () => []),
      onDownload: vi.fn(),
      onCancelDownload: vi.fn(),
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
    catalog: null,
    onRetryCatalog: vi.fn(),
    onCheckBasemapUpdates: vi.fn(async () => []),
    onDownload: vi.fn(),
    onCancelDownload: vi.fn(),
    airspace: { generation: 0, sources: [] },
    waypoints: { generation: 1, sources: [{ type: 'disabled', sourceName: 'local.cup' }] },
    activation: activation(),
    onRemove: vi.fn(),
    importer,
  });
  await openImport();
  await expect.element(page.getByRole('alertdialog')).toBeVisible();
  expect(importer.importDataFile).not.toHaveBeenCalled();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect.poll(() => importer.discardDataFile.mock.calls).toEqual([['1']]);
  await expect.element(page.getByRole('button', { name: 'Import custom file…' })).toHaveFocus();
  await openImport();
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
    catalog: null,
    onRetryCatalog: vi.fn(),
    onCheckBasemapUpdates: vi.fn(async () => []),
    onDownload: vi.fn(),
    onCancelDownload: vi.fn(),
    importer,
    activation: activation(),
    onRemove: vi.fn(),
    airspace: { generation: 1, sources: [{ type: 'disabled', sourceName: 'local.cup' }] },
    waypoints: { generation: 0, sources: [] },
  });
  await page.getByRole('button', { name: 'Add data' }).click();
  let add = page.getByRole('button', { name: 'Import custom file…' });
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
    catalog: null,
    onRetryCatalog: vi.fn(),
    onCheckBasemapUpdates: vi.fn(async () => []),
    onDownload: vi.fn(),
    onCancelDownload: vi.fn(),
    importer,
    activation: activation(),
    onRemove: vi.fn(),
    airspace: { generation: 0, sources: [] },
    waypoints: { generation: 0, sources: [] },
  });
  await openImport();
  await view.unmount();
  result.resolve({ selectionId: '3', sourceName: 'local.txt', dataType: 'airspace' });
  await expect.poll(() => importer.discardDataFile.mock.calls).toEqual([['3']]);
  expect(importer.importDataFile).not.toHaveBeenCalled();
});

it('moves Add data from the footer to the header above 544px', async () => {
  let previous = { width: window.innerWidth, height: window.innerHeight };
  await render(DataLibrary, {
    catalog: null,
    onRetryCatalog: vi.fn(),
    onCheckBasemapUpdates: vi.fn(async () => []),
    onDownload: vi.fn(),
    onCancelDownload: vi.fn(),
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
      catalog: null,
      onRetryCatalog: vi.fn(),
      onCheckBasemapUpdates: vi.fn(async () => []),
      onDownload: vi.fn(),
      onCancelDownload: vi.fn(),
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
    await openImport();
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
      expect(main.checkVisibility()).toBe(false);
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
    if (fails) {
      await expect.element(page.getByRole('alert')).toBeVisible();
      await page.getByRole('button', { name: 'Back to data', exact: true }).click();
    }
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

it.each(['basemap', 'terrain'] as const)(
  'shows %s activation and confirms removal',
  async (type) => {
    let extension = type === 'basemap' ? 'mbtiles' : 'terrain';
    let onRemove = vi
      .fn()
      .mockRejectedValueOnce(new Error('storage failed'))
      .mockResolvedValue(undefined);
    let inventory = {
      generation: 0,
      sources: [
        { sourceName: `z.${extension}`, type: 'disabled' as const },
        { sourceName: `a.${extension}`, type: 'active' as const },
        { sourceName: `broken.${extension}`, type: 'unavailable' as const },
      ],
    };
    let view = await render(DataLibrary, {
      catalog: null,
      onRetryCatalog: vi.fn(),
      onCheckBasemapUpdates: vi.fn(async () => []),
      onDownload: vi.fn(),
      onCancelDownload: vi.fn(),
      importer: new FakeClient(),
      activation: activation(),
      onRemove,
      airspace: { generation: 0, sources: [] },
      waypoints: { generation: 0, sources: [] },
      [type === 'basemap' ? 'basemaps' : 'terrain']: inventory,
    });
    await expect
      .element(
        page.getByRole('heading', {
          name: type === 'basemap' ? 'Basemap' : 'Terrain',
          exact: true,
        }),
      )
      .toBeVisible();
    expect(
      [...document.querySelectorAll('.filename')].map((element) => element.textContent),
    ).toEqual([`a.${extension}`, `broken.${extension}`, `z.${extension}`]);
    await page.getByRole('button', { name: new RegExp(`^a\\.${extension}`) }).click();
    await expect.element(page.getByRole('switch')).toBeChecked();
    await expect.element(page.getByRole('button', { name: 'Remove from device' })).toBeVisible();
    await expect
      .element(page.getByRole('dialog').getByText('Enabled', { exact: true }))
      .toBeVisible();
    await userEvent.keyboard('{Escape}');
    await page.getByRole('button', { name: new RegExp(`^broken\\.${extension}`) }).click();
    await expect
      .element(page.getByRole('dialog').getByText('Could not load the file.'))
      .toBeVisible();
    await userEvent.keyboard('{Escape}');
    await page.getByRole('button', { name: new RegExp(`^z\\.${extension}`) }).click();
    await expect.element(page.getByRole('switch')).not.toBeChecked();
    await page.getByRole('button', { name: 'Remove from device' }).click();
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    expect(onRemove).not.toHaveBeenCalled();
    await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
    await page.getByRole('button', { name: new RegExp(`^z\\.${extension}`) }).click();
    await page.getByRole('button', { name: 'Remove from device' }).click();
    await page.getByRole('button', { name: 'Remove', exact: true }).click();
    await expect.element(page.getByRole('alert')).toHaveTextContent('Could not remove the file.');
    await page.getByRole('button', { name: 'Remove', exact: true }).click();
    expect(onRemove.mock.calls).toEqual([
      [type, `z.${extension}`],
      [type, `z.${extension}`],
    ]);
    await expect.element(page.getByRole('alertdialog')).not.toBeInTheDocument();
    await view.rerender({
      [type === 'basemap' ? 'basemaps' : 'terrain']: { generation: 1, sources: [] },
    });
    await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
    await expect.element(page.getByText('No data on this device')).toBeVisible();
  },
);

it.each([
  ['basemaps', 'basemapError', 'Loading basemaps…', 'Could not load basemap inventory.'],
  ['terrain', 'terrainError', 'Loading terrain…', 'Could not load terrain inventory.'],
] as const)(
  'distinguishes pending and failed %s from an empty library',
  async (dataset, error, loading, failure) => {
    let view = await render(DataLibrary, {
      catalog: null,
      onRetryCatalog: vi.fn(),
      onCheckBasemapUpdates: vi.fn(async () => []),
      onDownload: vi.fn(),
      onCancelDownload: vi.fn(),
      importer: new FakeClient(),
      activation: activation(),
      onRemove: vi.fn(),
      airspace: { generation: 0, sources: [] },
      waypoints: { generation: 0, sources: [] },
      [dataset]: null,
    });
    await expect.element(page.getByRole('status')).toHaveTextContent(loading);
    await expect.element(page.getByText('No data on this device')).not.toBeInTheDocument();
    await view.rerender({ [error]: true });
    await expect.element(page.getByRole('alert')).toHaveTextContent(failure);
    await expect.element(page.getByRole('status')).not.toBeInTheDocument();
    await expect.element(page.getByText('No data on this device')).not.toBeInTheDocument();
  },
);

it('keeps accessible IDs unique across Data library instances', async () => {
  for (let name of ['first.txt', 'second.txt']) {
    await render(DataLibrary, {
      catalog: null,
      onRetryCatalog: vi.fn(),
      onCheckBasemapUpdates: vi.fn(async () => []),
      onDownload: vi.fn(),
      onCancelDownload: vi.fn(),
      importer: new FakeClient(),
      activation: activation(),
      onRemove: vi.fn(),
      airspace: {
        generation: 0,
        sources: [{ sourceName: name, type: 'active', airspaceCount: 1 }],
      },
      waypoints: { generation: 0, sources: [{ sourceName: 'local.cup', type: 'disabled' }] },
      basemaps: { generation: 0, sources: [{ sourceName: 'local.mbtiles', type: 'active' }] },
      terrain: { generation: 0, sources: [{ sourceName: 'local.terrain', type: 'active' }] },
    });
  }
  let sections = [...document.querySelectorAll('section[aria-labelledby]')];
  expect(sections).toHaveLength(8);
  let headingIds = sections.map((section) => {
    let heading = section.querySelector('h2')!;
    expect(section.getAttribute('aria-labelledby')).toBe(heading.id);
    expect(document.getElementById(heading.id)).toBe(heading);
    return heading.id;
  });
  expect(new Set(headingIds).size).toBe(8);

  let hintIds = [];
  for (let name of ['first.txt', 'second.txt']) {
    await page.getByRole('button', { name: new RegExp(`^${name}`) }).click();
    let dialog = page.getByRole('dialog');
    let hint = dialog
      .getByText('Kept on the device but not drawn or used in calculations when off')
      .element();
    let toggle = dialog.getByRole('switch').element();
    expect(toggle.getAttribute('aria-describedby')).toBe(hint.id);
    expect(document.getElementById(hint.id)).toBe(hint);
    expect(getComputedStyle(hint).display).toBe('block');
    hintIds.push(hint.id);
    await userEvent.keyboard('{Escape}');
  }
  expect(new Set(hintIds).size).toBe(2);
});

it('shows terrain activation details and keeps them current', async () => {
  let view = await render(DataLibrary, {
    catalog: null,
    onRetryCatalog: vi.fn(),
    onCheckBasemapUpdates: vi.fn(async () => []),
    onDownload: vi.fn(),
    onCancelDownload: vi.fn(),
    importer: new FakeClient(),
    activation: activation(),
    onRemove: vi.fn(),
    airspace: { generation: 0, sources: [] },
    waypoints: { generation: 0, sources: [] },
    basemaps: { generation: 0, sources: [{ sourceName: 'local.mbtiles', type: 'active' }] },
    terrain: {
      generation: 0,
      sources: [
        { sourceName: 'z.terrain', type: 'disabled' },
        { sourceName: 'a.terrain', type: 'active' },
        { sourceName: 'broken.terrain', type: 'unavailable' },
      ],
    },
  });
  expect(
    [...document.querySelectorAll('section h2')].map((element) => element.textContent),
  ).toEqual(['Basemap', 'Terrain']);
  expect([...document.querySelectorAll('.filename')].map((element) => element.textContent)).toEqual(
    ['local.mbtiles', 'a.terrain', 'broken.terrain', 'z.terrain'],
  );
  await page.getByRole('button', { name: /^a.terrain/ }).click();
  let dialog = page.getByRole('dialog');
  await expect.element(dialog.getByRole('switch')).toBeChecked();
  await expect.element(dialog.getByRole('button', { name: 'Remove from device' })).toBeVisible();
  await expect.element(dialog.getByText('Imported', { exact: true })).not.toBeInTheDocument();
  await userEvent.keyboard('{Escape}');
  await page.getByRole('button', { name: /^z.terrain/ }).click();
  await expect.element(dialog.getByRole('switch')).not.toBeChecked();
  await userEvent.keyboard('{Escape}');
  await page.getByRole('button', { name: /^broken.terrain/ }).click();
  await expect.element(dialog.getByText('Could not load the file.')).toBeVisible();
  await view.rerender({
    terrain: { generation: 1, sources: [{ sourceName: 'broken.terrain', type: 'disabled' }] },
  });
  await expect.element(dialog.getByRole('switch')).not.toBeChecked();
  await expect.element(dialog.getByText('Could not load the file.')).not.toBeInTheDocument();
  await view.rerender({ terrain: { generation: 2, sources: [] } });
  await expect.element(dialog).not.toBeInTheDocument();
  await expect
    .element(page.getByRole('heading', { name: 'Terrain', exact: true }))
    .not.toBeInTheDocument();
});

async function openImport() {
  let add = page.getByRole('button', { name: 'Add data', exact: true });
  if (add.elements().length) await add.click();
  await page.getByRole('button', { name: 'Import custom file…', exact: true }).click();
}

function downloadProps() {
  return {
    catalog: null,
    onRetryCatalog: vi.fn(),
    onCheckBasemapUpdates: vi.fn(async () => []),
    onDownload: vi.fn().mockResolvedValue(undefined),
    onCancelDownload: vi.fn().mockResolvedValue(undefined),
    importer: new FakeClient(),
    activation: activation(),
    onRemove: vi.fn(),
    airspace: { generation: 0, sources: [] },
    waypoints: { generation: 0, sources: [] },
  };
}

it('shows live download rows without opening details for uninstalled files', async () => {
  let options = downloadProps();
  let path = 'Europe/Malta.mbtiles';
  let screen = await render(DataLibrary, {
    ...options,
    downloads: [{ path, type: 'downloading', downloaded: 1_000_000, total: 2_000_000 }],
  });
  await expect.element(page.getByText('Downloading · 1 MB of 2 MB', { exact: true })).toBeVisible();
  await expect.element(page.getByRole('progressbar')).toHaveAttribute('value', '1000000');
  await expect.element(page.getByRole('button', { name: /^Malta/ })).not.toBeInTheDocument();
  await expect.element(page.getByText('On device', { exact: true })).not.toBeInTheDocument();
  await page.getByRole('button', { name: 'Cancel download: Malta.mbtiles', exact: true }).click();
  expect(options.onCancelDownload).toHaveBeenCalledExactlyOnceWith(path);
  await screen.rerender({ downloads: [] });
  await expect.element(page.getByText('Malta.mbtiles', { exact: true })).not.toBeInTheDocument();
});

it('combines an installed disabled file with its download and keeps details available', async () => {
  let path = 'Europe/Malta.mbtiles';
  let screen = await render(DataLibrary, {
    ...downloadProps(),
    basemaps: { generation: 1, sources: [{ sourceName: `enroute/${path}`, type: 'disabled' }] },
    downloads: [{ path, type: 'queued' }],
  });
  expect(page.getByText('Malta.mbtiles', { exact: true }).elements()).toHaveLength(1);
  await expect.element(page.getByText('On device', { exact: true })).toBeVisible();
  await expect.element(page.getByText('Disabled', { exact: true })).toBeVisible();
  await expect.element(page.getByText('Queued', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: /^Malta.mbtiles/ }).click();
  await expect.element(page.getByRole('dialog')).toBeVisible();
  await page.getByRole('button', { name: 'Close', exact: true }).click();
  await screen.rerender({ downloads: [] });
  await expect.element(page.getByText('Queued', { exact: true })).not.toBeInTheDocument();
  await expect.element(page.getByText('Malta.mbtiles', { exact: true })).toBeVisible();
});

it('keeps failed rows, reports command errors, and retries the exact path', async () => {
  let options = downloadProps();
  let path = 'Europe/Malta.mbtiles';
  let screen = await render(DataLibrary, { ...options, downloads: [{ path, type: 'failed' }] });
  vi.mocked(options.onDownload).mockRejectedValueOnce(new Error('IPC failed'));
  let retry = page.getByRole('button', { name: 'Retry download: Malta.mbtiles', exact: true });
  await retry.click();
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not start the downloads. Try again.');
  await expect.element(page.getByText('Download failed', { exact: true })).toBeVisible();
  await retry.click();
  expect(options.onDownload).toHaveBeenLastCalledWith([path]);
  await expect.element(page.getByRole('alert')).not.toBeInTheDocument();
  await expect.element(page.getByText('Download failed', { exact: true })).toBeVisible();
  await screen.rerender({ downloads: [{ path, type: 'queued' }] });
  vi.mocked(options.onCancelDownload).mockRejectedValueOnce(new Error('IPC failed'));
  await page.getByRole('button', { name: 'Cancel download: Malta.mbtiles', exact: true }).click();
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not cancel the download. Try again.');
});

it.each([false, true])(
  'returns after acceptance and row delivery with statusFirst=%s',
  async (statusFirst) => {
    let options = downloadProps();
    let accepted = Promise.withResolvers<void>();
    options.onDownload.mockReturnValue(accepted.promise);
    let path = 'Europe/Malta.mbtiles';
    let screen = await render(DataLibrary, {
      ...options,
      onCheckBasemapUpdates: vi.fn().mockResolvedValue([]),
      catalog: {
        cached: {
          checkedAt: 0,
          entries: [
            {
              path,
              countryCode: 'MT',
              continent: 'europe',
              size: 1_000_000,
              publicationDate: '2026-09-08',
            },
          ],
        },
        refreshing: false,
        error: false,
      },
    });
    await page.getByRole('button', { name: 'Add data', exact: true }).click();
    await page.getByRole('button', { name: 'Malta', exact: true }).click();
    await page.getByRole('checkbox', { name: 'Malta', exact: true }).click();
    await page.getByRole('button', { name: /^Download/ }).click();
    expect(options.onDownload).toHaveBeenCalledExactlyOnceWith([path]);
    if (statusFirst) await screen.rerender({ downloads: [{ path, type: 'queued' }] });
    await expect.element(page.getByRole('heading', { name: 'Malta', exact: true })).toBeVisible();
    accepted.resolve();
    await accepted.promise;
    if (!statusFirst) {
      await expect.element(page.getByRole('heading', { name: 'Malta', exact: true })).toBeVisible();
      await expect.element(page.getByRole('button', { name: /^Download/ })).toBeDisabled();
      await screen.rerender({ downloads: [{ path, type: 'queued' }] });
    }
    await expect.element(page.getByRole('heading', { name: 'Data', exact: true })).toBeVisible();
    await expect
      .element(page.getByRole('button', { name: 'Cancel download: Malta.mbtiles', exact: true }))
      .toBeVisible();
    await expect.element(page.getByRole('main')).toHaveFocus();
    await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
  },
);
