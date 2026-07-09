import type { PageLoad } from './$types';
import { m } from '$lib/paraglide/messages.js';
import { parseCoord, queryAt } from '$lib/map/query';

// The tapped coordinate lives in the URL (`@lat,lng`), so this list is
// deep-linkable and survives a refresh. In the real app the core answers this
// via `query_at`; here it's a deterministic mock.
export const load: PageLoad = ({ params }) => {
  let coord = parseCoord(params.coord);
  return {
    title: () => m.whats_here_title(),
    back: null,
    coord: params.coord,
    features: queryAt(coord),
  };
};
