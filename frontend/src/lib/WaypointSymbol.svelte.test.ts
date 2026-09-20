import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';

import outlanding from '../../../libs/updraft_sprites/sprites/outlanding-bg.svg?url';
import airfield from '../../../libs/updraft_sprites/sprites/waypoint-airfield.svg?url';
import mountain from '../../../libs/updraft_sprites/sprites/waypoint-mountain-top.svg?url';
import WaypointSymbol from './WaypointSymbol.svelte';

it.each([
  [2, airfield],
  [3, outlanding],
  [4, airfield],
  [5, airfield],
] as const)(
  'shows a runway for landable kind %s only when its direction is available',
  async (kind, image) => {
    let component = await render(WaypointSymbol, { kind, runwayDirection: 90 });
    let symbol = document.querySelector('.waypoint-symbol')!;
    expect(symbol).toHaveAttribute('aria-hidden', 'true');
    expect(symbol.querySelector('.shape')).toHaveStyle({ maskImage: `url("${image}")` });
    expect(symbol.querySelector('.runway')).toHaveStyle({ transform: 'rotate(90deg)' });

    await component.rerender({ runwayDirection: undefined });
    expect(symbol.querySelector('.runway')).toBeNull();
    await component.rerender({ kind: 7, runwayDirection: 90 });
    expect(getComputedStyle(symbol.querySelector('.shape')!).maskImage).toBe(`url("${mountain}")`);
    expect(symbol.querySelector('.runway')).toBeNull();
  },
);

it.each([0, 1, 6, 20, 21, 99])('does not show a runway for nonlandable kind %s', async (kind) => {
  await render(WaypointSymbol, { kind, runwayDirection: 90 });
  let symbol = document.querySelector('.waypoint-symbol')!;
  expect(symbol).not.toHaveClass('landable');
  expect(symbol.querySelector('.runway')).toBeNull();
});
