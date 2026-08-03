import type { Map } from 'maplibre-gl';

/** Holds the shared MapLibre map instance and camera state for one application session. */
export class MapState {
  map = $state.raw<Map | undefined>(undefined);
  center = $state({ lat: 50.823, lng: 6.186 });
  zoom = $state(11);
  bearing = $state(0);
  pitch = $state(0);
  followMode = $state(true);
}
