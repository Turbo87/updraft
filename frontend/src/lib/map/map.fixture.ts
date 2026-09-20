import type { ComponentProps } from 'svelte';
import type { MapState } from '$lib/map-state.svelte';
import type Map from './Map.svelte';

import { instrumentsFixture } from '$lib/instruments.fixture';
import { TrafficStore } from '$lib/stores/traffic.svelte';

export function mapProps(mapState: MapState) {
  return {
    hillshadeDirection: 'fixed',
    instruments: instrumentsFixture(),
    mapState,
    traffic: new TrafficStore(),
    units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
    airspace: { generation: 0, sources: [] },
    testMode: true,
  } satisfies ComponentProps<typeof Map>;
}
