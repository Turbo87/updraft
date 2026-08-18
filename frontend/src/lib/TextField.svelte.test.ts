import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';
import 'virtual:uno.css';

import TextField from './TextField.svelte';

describe('TextField.svelte', () => {
  it('associates the label and hint with the input', async () => {
    render(TextField, {
      label: 'Host',
      hint: 'Host name or IP address of the instrument.',
      value: '192.168.4.1',
    });

    let input = page.getByLabelText('Host');
    let hint = page.getByText('Host name or IP address of the instrument.');

    await expect.element(input).toHaveValue('192.168.4.1');
    await expect.element(input).toHaveAttribute('aria-describedby', hint.element().id);
  });

  it('replaces the hint with an accessible error', async () => {
    render(TextField, {
      label: 'Port',
      hint: 'The TCP port.',
      error: 'Enter a whole-number port from 1 to 65535.',
      inputmode: 'numeric',
      value: '70000',
    });

    let input = page.getByLabelText('Port');
    let error = page.getByRole('alert');

    await expect.element(input).toHaveAttribute('aria-invalid', 'true');
    await expect.element(input).toHaveAttribute('aria-describedby', error.element().id);
    await expect.element(page.getByText('The TCP port.')).not.toBeInTheDocument();
    expect(error.element().querySelector('.i-mdi-alert-circle-outline')).not.toBeNull();
  });

  it('keeps the error icon within the first line of text', () => {
    render(TextField, {
      label: 'Port',
      error: 'Enter a whole-number port from 1 to 65535.',
      value: '70000',
    });

    let error = page.getByRole('alert').element();
    let icon = error.querySelector('.i-mdi-alert-circle-outline')!;
    let iconBounds = icon.getBoundingClientRect();
    let textRange = document.createRange();
    textRange.selectNodeContents(error.lastElementChild!);
    let firstLineBounds = textRange.getClientRects()[0];

    // One CSS pixel covers opposite subpixel rounding without allowing a visible line shift.
    expect(iconBounds.top).toBeGreaterThanOrEqual(firstLineBounds.top - 1);
    expect(iconBounds.bottom).toBeLessThanOrEqual(firstLineBounds.bottom + 1);
  });

  it('uses tabular figures for numeric input', () => {
    render(TextField, { label: 'Port', inputmode: 'numeric', value: '2000' });

    let input = page.getByLabelText('Port').element() as HTMLInputElement;

    expect(input.inputMode).toBe('numeric');
    expect(getComputedStyle(input).fontVariantNumeric).toContain('tabular-nums');
  });

  it('keeps the text inset stable when focused', async () => {
    render(TextField, { label: 'Host', value: '192.168.4.1' });

    let input = page.getByLabelText('Host');
    let inputElement = input.element() as HTMLInputElement;
    let restingStyle = getComputedStyle(inputElement);
    let restingInset =
      parseFloat(restingStyle.borderLeftWidth) + parseFloat(restingStyle.paddingLeft);

    await input.click();

    let focusedStyle = getComputedStyle(inputElement);
    let focusedInset =
      parseFloat(focusedStyle.borderLeftWidth) + parseFloat(focusedStyle.paddingLeft);
    expect(document.activeElement).toBe(inputElement);
    expect(focusedInset).toBe(restingInset);
  });

  it('forwards native input attributes', async () => {
    render(TextField, { disabled: true, label: 'Service UUID', name: 'serviceUuid' });

    let input = page.getByLabelText('Service UUID');

    await expect.element(input).toBeDisabled();
    await expect.element(input).toHaveAttribute('name', 'serviceUuid');
  });
});
