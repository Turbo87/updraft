import { createRawSnippet } from 'svelte';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import ListRow from './ListRow.svelte';

const connected = createRawSnippet(() => ({ render: () => '<span>Connected</span>' }));

describe('ListRow.svelte', () => {
  it('renders the full navigating row as a 56-pixel link', async () => {
    render(ListRow, {
      href: '/settings/airspace',
      icon: 'i-mdi-vector-square',
      label: 'Airspace',
      size: 'large',
      value: 'Germany 2026',
    });

    let link = page.getByRole('link', { name: 'Airspace Germany 2026' });
    await expect.element(link).toHaveAttribute('href', '/settings/airspace');
    expect(getComputedStyle(link.element()).height).toBe('56px');
    expect(link.element().querySelector('.i-mdi-vector-square')).not.toBeNull();
    expect(link.element().querySelector('.i-mdi-chevron-right')).not.toBeNull();
  });

  it('renders a read-only value in a 48-pixel row without a chevron', () => {
    render(ListRow, { label: 'Altitude', numeric: true, value: '1245 m MSL' });

    let row = page.getByText('Altitude').element().closest('.list-row');

    expect(row).not.toBeNull();
    expect(getComputedStyle(row!).height).toBe('48px');
    expect(row!.querySelector('.numeric')).not.toBeNull();
    expect(row!.querySelector('.i-mdi-chevron-right')).toBeNull();
  });

  it('renders trailing content instead of a value', async () => {
    render(ListRow, { label: 'TCP · 192.168.4.1:2000', trailing: connected });

    await expect.element(page.getByText('Connected')).toBeVisible();
  });

  it('disables a navigating row without exposing a link target or chevron', async () => {
    render(ListRow, {
      disabled: true,
      href: '/settings/devices',
      label: 'Bluetooth SPP',
      size: 'large',
      value: 'Unsupported',
    });

    let row = page.getByText('Bluetooth SPP').element().closest('.list-row');

    expect(row).not.toBeNull();
    await expect.element(page.getByText('Bluetooth SPP')).toBeVisible();
    expect(row!.getAttribute('href')).toBeNull();
    expect(row!.getAttribute('aria-disabled')).toBe('true');
    expect(getComputedStyle(row!).minHeight).toBe('56px');
    expect(row!.querySelector('.i-mdi-chevron-right')).toBeNull();
  });

  it('right-aligns a lone chevron with consistent horizontal padding', () => {
    render(ListRow, { href: '/settings/about', label: 'About', size: 'large' });

    let row = page.getByText('About').element().closest('.list-row');
    let chevron = row?.querySelector<HTMLElement>('.i-mdi-chevron-right');

    expect(row).not.toBeNull();
    expect(chevron).not.toBeNull();

    let rowBounds = row!.getBoundingClientRect();
    let rowStyle = getComputedStyle(row!);
    expect(rowStyle.paddingLeft).toBe('20px');
    expect(rowStyle.paddingRight).toBe('20px');
    expect(rowBounds.right - chevron!.getBoundingClientRect().right).toBe(21);
  });

  it('uses the same horizontal padding for a read-only value', () => {
    render(ListRow, { label: 'Language', value: 'English' });

    let row = page.getByText('Language').element().closest('.list-row');
    let value = page.getByText('English').element();

    expect(row).not.toBeNull();

    let rowBounds = row!.getBoundingClientRect();
    let rowStyle = getComputedStyle(row!);
    expect(rowStyle.paddingLeft).toBe('20px');
    expect(rowStyle.paddingRight).toBe('20px');
    expect(rowBounds.right - value.getBoundingClientRect().right).toBe(21);
  });
});
