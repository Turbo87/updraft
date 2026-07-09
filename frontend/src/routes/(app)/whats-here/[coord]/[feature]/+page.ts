import type { PageLoad } from './$types';
import { error } from '@sveltejs/kit';
import { m } from '$lib/paraglide/messages.js';
import { parseCoord, getFeature } from '$lib/map/query';

export const load: PageLoad = ({ params }) => {
  let feature = getFeature(parseCoord(params.coord), params.feature);
  if (!feature) error(404, m.error_unknown_feature());
  return {
    // A feature name is data, not a translatable message, but keep the getter
    // shape the chrome expects.
    title: () => feature.name,
    back: `/whats-here/${params.coord}`,
    feature,
  };
};
