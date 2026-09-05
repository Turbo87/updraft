import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';

import airfield from '../../../libs/updraft_sprites/sprites/waypoint-airfield.svg?url';
import mountain from '../../../libs/updraft_sprites/sprites/waypoint-mountain-top.svg?url';
import WaypointSymbol from './WaypointSymbol.svelte';

it('shows a runway on landable symbols and updates to terrain symbols', async () => {
  let component = await render(WaypointSymbol, { kind: 2, runwayDirection: 90 });
  let symbol = document.querySelector('.waypoint-symbol')!;
  expect(symbol).toHaveAttribute('aria-hidden', 'true');
  expect(getComputedStyle(symbol.querySelector('.shape')!).maskImage).toBe(`url("${airfield}")`);
  expect(symbol.querySelector('.runway')).toHaveStyle({ transform: 'rotate(90deg)' });

  await component.rerender({ kind: 7 });
  expect(getComputedStyle(symbol.querySelector('.shape')!).maskImage).toBe(`url("${mountain}")`);
  expect(symbol.querySelector('.runway')).toBeNull();
});
