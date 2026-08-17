import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import '../app.css';

import ConfirmDialog from './ConfirmDialog.svelte';

const props = {
  open: true,
  title: 'Remove Germany 2026.txt?',
  description:
    'The file is deleted from this device and airspace disappears from the map. You can import it again later.',
  cancelLabel: 'Cancel',
  confirmLabel: 'Remove',
  onCancel: () => {},
  onConfirm: () => {},
};

describe('ConfirmDialog.svelte', () => {
  it('stays closed until opened', async () => {
    await render(ConfirmDialog, { ...props, open: false });

    await expect
      .element(page.getByRole('alertdialog', { name: props.title }))
      .not.toBeInTheDocument();
  });

  it('labels the modal and initially focuses the safe action', async () => {
    await render(ConfirmDialog, props);

    let dialog = page.getByRole('alertdialog', { name: props.title });
    let cancel = page.getByRole('button', { name: props.cancelLabel });

    await expect.element(dialog).toBeVisible();
    await expect.element(dialog).toHaveAttribute('aria-modal', 'true');
    await expect.element(dialog).toHaveAccessibleDescription(props.description);
    await expect.element(cancel).toHaveFocus();

    await userEvent.keyboard('{Escape}');
  });

  it('reports cancellation', async () => {
    let onCancel = vi.fn();
    await render(ConfirmDialog, { ...props, onCancel });

    await page.getByRole('button', { name: props.cancelLabel }).click();
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it('reports confirmation and waits for its parent to close', async () => {
    let onConfirm = vi.fn();
    let view = await render(ConfirmDialog, { ...props, onConfirm });

    await page.getByRole('button', { name: props.confirmLabel }).click();
    expect(onConfirm).toHaveBeenCalledOnce();
    await expect.element(page.getByRole('alertdialog', { name: props.title })).toBeInTheDocument();

    await view.rerender({ ...props, onConfirm, open: false });
    await expect
      .element(page.getByRole('alertdialog', { name: props.title }))
      .not.toBeInTheDocument();
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it('traps focus and cancels with Escape', async () => {
    let onCancel = vi.fn();
    await render(ConfirmDialog, { ...props, onCancel });
    let cancel = page.getByRole('button', { name: props.cancelLabel });
    let confirm = page.getByRole('button', { name: props.confirmLabel });

    await expect.element(cancel).toHaveFocus();
    await userEvent.keyboard('{Shift>}{Tab}{/Shift}');
    await expect.element(confirm).toHaveFocus();
    await userEvent.keyboard('{Tab}');
    await expect.element(cancel).toHaveFocus();
    await userEvent.keyboard('{Escape}');

    expect(onCancel).toHaveBeenCalledOnce();
  });

  it('restores focus when the modal unmounts', async () => {
    let trigger = document.createElement('button');
    trigger.textContent = 'Open confirmation';
    document.body.append(trigger);
    trigger.focus();

    let view = await render(ConfirmDialog, props);
    await expect.element(page.getByRole('button', { name: props.cancelLabel })).toHaveFocus();

    await page.getByRole('button', { name: props.cancelLabel }).click();

    expect(document.activeElement).toBe(trigger);
    await view.unmount();
    trigger.remove();
  });
});
