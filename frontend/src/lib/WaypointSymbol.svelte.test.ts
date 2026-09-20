import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';

import outlanding from '../../../libs/updraft_sprites/sprites/outlanding-bg.svg?url';
import airfield from '../../../libs/updraft_sprites/sprites/waypoint-airfield.svg?url';
import mountain from '../../../libs/updraft_sprites/sprites/waypoint-mountain-top.svg?url';
import { WAYPOINT_KIND } from './waypoints';
import WaypointSymbol from './WaypointSymbol.svelte';

it.each([
  [WAYPOINT_KIND.AIRFIELD_GRASS, airfield],
  [WAYPOINT_KIND.OUTLANDING, outlanding],
  [WAYPOINT_KIND.GLIDING_AIRFIELD, airfield],
  [WAYPOINT_KIND.AIRFIELD_SOLID, airfield],
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
    await component.rerender({ kind: WAYPOINT_KIND.MOUNTAIN_TOP, runwayDirection: 90 });
    expect(getComputedStyle(symbol.querySelector('.shape')!).maskImage).toBe(`url("${mountain}")`);
    expect(symbol.querySelector('.runway')).toBeNull();
  },
);

it.each([
  WAYPOINT_KIND.UNKNOWN,
  WAYPOINT_KIND.WAYPOINT,
  WAYPOINT_KIND.MOUNTAIN_PASS,
  WAYPOINT_KIND.PG_TAKEOFF,
  WAYPOINT_KIND.PG_LANDING_ZONE,
  99,
])('does not show a runway for nonlandable kind %s', async (kind) => {
  await render(WaypointSymbol, { kind, runwayDirection: 90 });
  let symbol = document.querySelector('.waypoint-symbol')!;
  expect(symbol).not.toHaveClass('landable');
  expect(symbol.querySelector('.runway')).toBeNull();
});
