import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import ExternalDeviceForm from './ExternalDeviceForm.svelte';

describe('ExternalDeviceForm.svelte', () => {
  it('creates a TCP device with a trimmed host and numeric port', async () => {
    let onSave = vi.fn(async () => {});
    render(ExternalDeviceForm, { onSave });

    await page.getByLabelText('Host').fill('  flight-recorder.local  ');
    await page.getByLabelText('Port').fill('4353');
    await page.getByRole('button', { name: 'Add external device' }).click();

    expect(onSave).toHaveBeenCalledExactlyOnceWith({
      type: 'tcp',
      host: 'flight-recorder.local',
      port: 4353,
    });
  });

  it('identifies invalid TCP fields without saving', async () => {
    let onSave = vi.fn(async () => {});
    render(ExternalDeviceForm, { onSave });

    await page.getByLabelText('Host').fill('   ');
    await page.getByLabelText('Port').fill('65536');
    await page.getByRole('button', { name: 'Add external device' }).click();

    await expect.element(page.getByText('Enter a host.')).toBeInTheDocument();
    await expect
      .element(page.getByText('Enter a whole-number port from 1 to 65535.'))
      .toBeInTheDocument();
    expect(onSave).not.toHaveBeenCalled();
  });

  it('rejects ports outside the decimal integer range', async () => {
    let onSave = vi.fn(async () => {});
    render(ExternalDeviceForm, { onSave });
    await page.getByLabelText('Host').fill('192.0.2.1');

    for (let port of ['0', '65536', '1.5', 'not-a-port']) {
      await page.getByLabelText('Port').fill(port);
      await page.getByRole('button', { name: 'Add external device' }).click();
      await expect
        .element(page.getByText('Enter a whole-number port from 1 to 65535.'))
        .toBeInTheDocument();
    }

    expect(onSave).not.toHaveBeenCalled();
  });

  it('prefills and edits a TCP device', async () => {
    let onSave = vi.fn(async () => {});
    render(ExternalDeviceForm, {
      device: { deviceId: 4, enabled: false, type: 'tcp', host: '192.0.2.1', port: 4353 },
      onSave,
    });

    await expect.element(page.getByLabelText('Host')).toHaveValue('192.0.2.1');
    await expect.element(page.getByLabelText('Port')).toHaveValue('4353');

    await page.getByLabelText('Host').fill('192.0.2.2');
    await page.getByLabelText('Port').fill('10110');
    await page.getByRole('button', { name: 'Save changes' }).click();

    expect(onSave).toHaveBeenCalledExactlyOnceWith({
      type: 'tcp',
      host: '192.0.2.2',
      port: 10110,
    });
  });

  it('disables Save while the command is pending', async () => {
    let finishSave = () => {};
    let pendingSave = new Promise<void>((resolve) => {
      finishSave = resolve;
    });
    render(ExternalDeviceForm, { onSave: () => pendingSave });

    await page.getByLabelText('Host').fill('192.0.2.1');
    await page.getByLabelText('Port').fill('4353');
    let saveButton = page.getByRole('button', { name: 'Add external device' });
    await saveButton.click();

    await expect.element(saveButton).toBeDisabled();
    finishSave();
    await expect.element(saveButton).toBeEnabled();
  });

  it('keeps the form open and shows an error when Save fails', async () => {
    render(ExternalDeviceForm, {
      onSave: async () => {
        throw new Error('driver stopped');
      },
    });

    await page.getByLabelText('Host').fill('192.0.2.1');
    await page.getByLabelText('Port').fill('4353');
    let saveButton = page.getByRole('button', { name: 'Add external device' });
    await saveButton.click();

    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Could not save this external device.');
    await expect.element(page.getByLabelText('Host')).toHaveValue('192.0.2.1');
    await expect.element(saveButton).toBeEnabled();
  });

  it('deletes an existing TCP device only after confirmation', async () => {
    let onDelete = vi.fn(async () => {});
    render(ExternalDeviceForm, {
      device: { deviceId: 4, enabled: true, type: 'tcp', host: '192.0.2.1', port: 4353 },
      onSave: async () => {},
      onDelete,
    });

    await page.getByRole('button', { name: 'Delete external device' }).click();

    await expect
      .element(page.getByRole('dialog', { name: 'Delete 192.0.2.1:4353?' }))
      .toBeInTheDocument();
    expect(onDelete).not.toHaveBeenCalled();

    await page.getByRole('button', { name: 'Delete', exact: true }).click();
    expect(onDelete).toHaveBeenCalledOnce();
  });

  it('keeps the confirmation open and shows an error when Delete fails', async () => {
    render(ExternalDeviceForm, {
      device: { deviceId: 4, enabled: true, type: 'tcp', host: '192.0.2.1', port: 4353 },
      onSave: async () => {},
      onDelete: async () => {
        throw new Error('driver stopped');
      },
    });

    await page.getByRole('button', { name: 'Delete external device' }).click();
    await page.getByRole('button', { name: 'Delete', exact: true }).click();

    await expect
      .element(page.getByRole('dialog', { name: 'Delete 192.0.2.1:4353?' }))
      .toBeInTheDocument();
    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Could not delete this external device.');
  });
});
