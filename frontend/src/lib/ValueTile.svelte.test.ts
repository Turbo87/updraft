import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import ValueTile from './ValueTile.svelte';

describe('ValueTile.svelte', () => {
  it('renders a labelled value with a trailing unit', () => {
    render(ValueTile, { label: 'Altitude', value: '1245', unit: 'm' });

    expect(page.getByText('Altitude')).toBeInTheDocument();
    expect(page.getByText('1245')).toBeInTheDocument();
    expect(page.getByText('m')).toBeInTheDocument();
  });

  it('marks stale values', () => {
    render(ValueTile, { label: 'Wind', value: '248', unit: '°', stale: true });

    expect(page.getByText('248').element().closest('.readout')).toHaveClass('stale');
  });

  it('stacks fractional units', () => {
    render(ValueTile, { label: 'Vario', value: '+1.8', unit: 'm/s' });

    let unit = page.getByLabelText('m/s').element();

    expect(unit).toHaveClass('stacked-unit');
    expect(unit.children).toHaveLength(3);
    expect(unit.children[0]).toHaveTextContent('m');
    expect(unit.children[2]).toHaveTextContent('s');
  });

  it('top-aligns degree units', () => {
    render(ValueTile, { label: 'Track', value: '024', unit: '°' });

    expect(page.getByText('°')).toHaveClass('degree-unit');
  });
});
