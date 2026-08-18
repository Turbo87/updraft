import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import StatusPill from './StatusPill.svelte';

describe('StatusPill.svelte', () => {
  it('renders a read-only status without an indicator by default', () => {
    render(StatusPill, { label: 'Connected', tone: 'success' });

    let pill = page.getByText('Connected').element().closest('.status-pill');

    expect(pill).not.toBeNull();
    expect(pill).toBeInstanceOf(HTMLSpanElement);
    expect(pill).toHaveClass('success');
    expect(pill!.querySelector('.indicator-slot')).toBeNull();
  });

  it('uses a loading indicator for a changing status', () => {
    render(StatusPill, { label: 'Connecting', loading: true, tone: 'caution' });

    let pill = page.getByText('Connecting').element().closest('.status-pill');

    expect(pill).not.toBeNull();
    expect(pill).toHaveClass('caution');
    expect(pill!.querySelector('.i-mdi-loading')).not.toBeNull();
  });

  it('keeps its indicator box and vertical position when loading changes', async () => {
    let view = await render(StatusPill, { label: 'Connecting', tone: 'caution' });
    let pill = page.getByText('Connecting').element().closest('.status-pill');
    let restingBounds = pill!.getBoundingClientRect();
    expect(pill!.querySelector('.indicator-slot')).toBeNull();

    await view.rerender({ label: 'Connecting', loading: true, tone: 'caution' });

    let loadingIndicator = pill!.querySelector<HTMLElement>('.indicator-slot');
    expect(loadingIndicator).not.toBeNull();
    expect(getComputedStyle(loadingIndicator!).width).toBe('16px');
    expect(getComputedStyle(loadingIndicator!).height).toBe('16px');
    expect(pill!.getBoundingClientRect().height).toBe(restingBounds.height);
    expect(pill!.getBoundingClientRect().top).toBe(restingBounds.top);
  });

  it('scales its geometry with the configured font size', () => {
    render(StatusPill, { label: 'Connecting', loading: true, tone: 'caution' });

    let pill = page.getByText('Connecting').element().closest<HTMLElement>('.status-pill');
    let indicator = pill!.querySelector<HTMLElement>('.indicator-slot');

    pill!.style.setProperty('--status-pill-font-size', '2rem');

    expect(getComputedStyle(pill!).fontSize).toBe('32px');
    expect(pill!.getBoundingClientRect().height).toBe(52);
    expect(getComputedStyle(indicator!).width).toBe('32px');
    expect(getComputedStyle(indicator!).height).toBe('32px');
  });

  it('uses a supplied icon', () => {
    render(StatusPill, {
      icon: 'i-mdi-alert-circle',
      label: 'Failed',
      tone: 'danger-subtle',
    });

    let pill = page.getByText('Failed').element().closest('.status-pill');

    expect(pill).not.toBeNull();
    expect(pill).toHaveClass('danger-subtle');
    expect(pill!.querySelector('.i-mdi-alert-circle')).not.toBeNull();
  });
});
