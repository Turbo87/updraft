import type { Navigation } from '$lib/protocol/generated/Navigation';

import { m } from '$lib/paraglide/messages';

export function navigationLabel(navigation: Pick<Navigation, 'target' | 'traffic'>): string {
  switch (navigation.target.type) {
    case 'waypoint':
      return navigation.target.name;
    case 'mapPosition':
      return m.navigation_map_position();
    case 'traffic':
      return navigation.traffic?.name ?? navigation.target.id;
  }
}
