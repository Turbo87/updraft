import type { Navigation } from '$lib/protocol/generated/Navigation';

import { m } from '$lib/paraglide/messages';

export function navigationLabel(
  navigation: Pick<Navigation, 'target' | 'traffic' | 'trafficName'>,
): string {
  switch (navigation.target.type) {
    case 'task':
      return m.task_heading();
    case 'waypoint':
      return navigation.target.name;
    case 'mapPosition':
      return m.navigation_map_position();
    case 'traffic':
      return navigation.traffic?.name ?? navigation.trafficName ?? navigation.target.id;
  }
}
