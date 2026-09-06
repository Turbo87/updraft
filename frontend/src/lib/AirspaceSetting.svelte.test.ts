import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import AirspaceSetting from './AirspaceSetting.svelte';

describe('AirspaceSetting.svelte', () => {
  it('shows an import action when no source is selected', async () => {
    render(AirspaceSetting, {
      status: { generation: 0, sources: [] },
      onImport: vi.fn(async () => ({ type: 'cancelled' as const })),
      onRemove: vi.fn(async () => {}),
    });

    let group = page.getByRole('group', { name: 'Airspace' });
    await expect.element(group).toBeVisible();
    expect(group.element().querySelector('.i-mdi-vector-square')).not.toBeNull();
    expect(getComputedStyle(group.getByText('Airspace', { exact: true }).element()).position).toBe(
      'absolute',
    );
    await expect.element(page.getByText('No airspace file selected.')).toBeVisible();
    await expect
      .element(
        page.getByText(
          'Import an OpenAir file to draw airspace on the map and enable airspace details.',
        ),
      )
      .toBeVisible();
    let importButton = page.getByRole('button', { name: 'Import' });
    await expect.element(importButton).toBeEnabled();
    expect(importButton.element().closest('footer')).not.toBeNull();
  });

  it('shows the active source and wires replacement', async () => {
    let onImport = vi.fn(async () => ({ type: 'cancelled' as const }));
    render(AirspaceSetting, {
      status: {
        generation: 1,
        sources: [{ type: 'active', sourceName: 'rheinland.txt', airspaceCount: 42 }],
      },
      onImport,
      onRemove: vi.fn(async () => {}),
    });

    await expect.element(page.getByRole('heading', { name: 'Current source' })).toBeVisible();
    await expect.element(page.getByText('File', { exact: true })).toBeVisible();
    await expect.element(page.getByText('rheinland.txt')).toBeVisible();
    await expect.element(page.getByText('Airspaces', { exact: true })).toBeVisible();
    await expect.element(page.getByText('42', { exact: true })).toBeVisible();
    await expect.element(page.getByText('State', { exact: true })).toBeVisible();
    await expect.element(page.getByText('Active', { exact: true })).toBeVisible();

    await expect
      .element(page.getByText('Importing a file replaces only the source with the same filename.'))
      .toBeVisible();

    let replace = page.getByRole('button', { name: 'Import' });
    expect(replace.element().closest('footer')).not.toBeNull();
    expect(
      page.getByRole('button', { name: 'Remove airspace source' }).element().closest('main'),
    ).not.toBeNull();

    await replace.click();
    expect(onImport).toHaveBeenCalledOnce();
  });

  it('removes the source only after confirmation', async () => {
    let onRemove = vi.fn(async () => {});
    render(AirspaceSetting, {
      status: {
        generation: 1,
        sources: [{ type: 'active', sourceName: 'rheinland.txt', airspaceCount: 42 }],
      },
      onImport: vi.fn(async () => ({ type: 'cancelled' as const })),
      onRemove,
    });

    let remove = page.getByRole('button', { name: 'Remove airspace source' });
    await remove.click();

    let dialog = page.getByRole('alertdialog', { name: 'Remove rheinland.txt?' });
    await expect.element(dialog).toBeVisible();
    await expect
      .element(dialog)
      .toHaveAccessibleDescription(
        'The file is deleted from this device. Its airspaces disappear from the map. Other files remain active.',
      );
    expect(onRemove).not.toHaveBeenCalled();

    await page.getByRole('button', { name: 'Cancel' }).click();
    await expect.element(dialog).not.toBeInTheDocument();
    expect(onRemove).not.toHaveBeenCalled();

    await remove.click();
    await page.getByRole('button', { name: 'Remove', exact: true }).click();
    expect(onRemove).toHaveBeenCalledOnce();
  });

  it('shows the stored filename', async () => {
    render(AirspaceSetting, {
      status: {
        generation: 0,
        sources: [{ type: 'active', sourceName: 'airspace.txt', airspaceCount: 1 }],
      },
      onImport: vi.fn(async () => ({ type: 'cancelled' as const })),
      onRemove: vi.fn(async () => {}),
    });

    await expect.element(page.getByText('airspace.txt')).toBeVisible();
    await expect.element(page.getByText('1', { exact: true })).toBeVisible();
  });

  it.each([
    ['readFailed', 'The airspace file could not be read.'],
    ['parseFailed', 'The airspace file could not be parsed.'],
    ['geometryFailed', 'The airspace geometry is invalid.'],
  ] as const)('shows the %s unavailable state', async (error, message) => {
    render(AirspaceSetting, {
      status: {
        generation: 0,
        sources: [{ type: 'unavailable', sourceName: 'broken.txt', error }],
      },
      onImport: vi.fn(async () => ({ type: 'cancelled' as const })),
      onRemove: vi.fn(async () => {}),
    });

    await expect.element(page.getByRole('heading', { name: 'Current source' })).toBeVisible();
    await expect.element(page.getByText('File', { exact: true })).toBeVisible();
    await expect.element(page.getByText('broken.txt')).toBeVisible();
    await expect.element(page.getByText('State', { exact: true })).toBeVisible();
    await expect.element(page.getByText('Unavailable', { exact: true })).toBeVisible();
    await expect.element(page.getByText(message)).toBeVisible();
    await expect.element(page.getByRole('button', { name: 'Import' })).toBeEnabled();
    await expect
      .element(page.getByRole('button', { name: 'Remove airspace source' }))
      .toBeEnabled();
  });

  it('disables all mutation controls while an action is pending', async () => {
    let finishImport: (result: { type: 'imported' }) => void = () => undefined;
    let pendingImport = new Promise<{ type: 'imported' }>((resolve) => {
      finishImport = resolve;
    });
    render(AirspaceSetting, {
      status: {
        generation: 1,
        sources: [{ type: 'active', sourceName: 'rheinland.txt', airspaceCount: 42 }],
      },
      onImport: vi.fn(() => pendingImport),
      onRemove: vi.fn(async () => {}),
    });

    let replace = page.getByRole('button', { name: 'Import' });
    let remove = page.getByRole('button', { name: 'Remove airspace source' });
    await replace.click();

    await expect.element(replace).toBeDisabled();
    await expect.element(remove).toBeDisabled();

    finishImport({ type: 'imported' });
    await expect.element(replace).toBeEnabled();
    await expect.element(remove).toBeEnabled();
  });

  it.each([
    { error: { kind: 'missingName' }, message: 'The selected file has no filename.' },
    {
      error: { kind: 'pickerFailed' },
      message: 'Could not open the file picker.',
    },
    {
      error: { kind: 'readFailed', sourceName: 'broken.txt' },
      message: 'Could not read the selected airspace file.',
    },
    {
      error: { kind: 'parseFailed', sourceName: 'broken.txt' },
      message: 'The selected airspace file could not be parsed.',
    },
    {
      error: { kind: 'geometryFailed', sourceName: 'broken.txt' },
      message: 'The selected airspace geometry is invalid.',
    },
    {
      error: { kind: 'storageFailed', sourceName: 'broken.txt' },
      message: 'Could not save the selected airspace file.',
    },
    {
      error: { kind: 'driverStopped', sourceName: 'broken.txt' },
      message: 'The airspace service is unavailable.',
    },
    {
      error: { kind: 'busy' },
      message: 'Another airspace change is already in progress.',
    },
  ] as const)('shows the localized $error.kind command error', async ({ error, message }) => {
    render(AirspaceSetting, {
      status: { generation: 0, sources: [] },
      onImport: vi.fn(async () => {
        throw error;
      }),
      onRemove: vi.fn(async () => {}),
    });

    let importButton = page.getByRole('button', { name: 'Import' });
    await importButton.click();

    await expect.element(page.getByRole('alert')).toHaveTextContent(message);
    await expect.element(importButton).toBeEnabled();
  });

  it('does not expose an unexpected backend error', async () => {
    render(AirspaceSetting, {
      status: { generation: 0, sources: [] },
      onImport: vi.fn(async () => {
        throw new Error('/private/path/airspace.txt');
      }),
      onRemove: vi.fn(async () => {}),
    });

    await page.getByRole('button', { name: 'Import' }).click();

    await expect.element(page.getByRole('alert')).toHaveTextContent('Could not update airspace.');
    await expect.element(page.getByText('/private/path/airspace.txt')).not.toBeInTheDocument();
  });
});

it('lists independent sources and removes only the confirmed filename', async () => {
  let onRemove = vi.fn<(name: string) => Promise<void>>(async () => {});
  render(AirspaceSetting, {
    status: {
      generation: 3,
      sources: [
        { type: 'active', sourceName: 'a.txt', airspaceCount: 1 },
        { type: 'active', sourceName: 'b.txt', airspaceCount: 2 },
      ],
    },
    onImport: vi.fn(async () => ({ type: 'cancelled' as const })),
    onRemove,
  });
  await expect.element(page.getByText('a.txt', { exact: true })).toBeVisible();
  await expect.element(page.getByText('b.txt', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Remove airspace source' }).nth(1).click();
  await expect.element(page.getByRole('alertdialog', { name: 'Remove b.txt?' })).toBeVisible();
  await page.getByRole('button', { name: 'Remove', exact: true }).click();
  expect(onRemove).toHaveBeenCalledExactlyOnceWith('b.txt');
});
