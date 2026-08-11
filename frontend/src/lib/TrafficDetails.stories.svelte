<script module lang="ts">
  import type { GpsInstruments } from '$lib/protocol/generated/GpsInstruments';
  import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';
  import type { TrafficUpdate } from '$lib/protocol/generated/TrafficUpdate';
  import type { TrafficSubscriber } from '$lib/stores/traffic.svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { fn } from 'storybook/test';

  import { InstrumentsStore } from '$lib/stores/instruments.svelte';
  import { TrafficStore } from '$lib/stores/traffic.svelte';
  import TrafficDetails from '../routes/traffic/[id]/TrafficDetails.svelte';

  const target = {
    id: 'flarm:DDX7A2',
    position: { latitudeDegrees: 50.82902, longitudeDegrees: 6.24417 },
    altitudeMslMeters: 1180,
    trafficType: 'glider',
    trackDegrees: 241,
    alarmLevel: 'none',
    stale: false,
  } satisfies PublishedTrafficTarget;

  const ownship = {
    position: { latitudeDegrees: 50.806, longitudeDegrees: 6.19 },
    altitudeMeters: 1115,
    groundSpeedMetersPerSecond: 31,
    trackDegrees: 225,
    fixTime: null,
    stale: false,
  } satisfies GpsInstruments;

  function createInstrumentsStore(gps: GpsInstruments | null): InstrumentsStore {
    let instruments = new InstrumentsStore();
    instruments.apply({
      topic: 'instruments',
      value: { gps, pressureAltitude: null, trueAirspeed: null, air: null },
    });
    return instruments;
  }

  function createTrafficStore(value: PublishedTrafficTarget | null): TrafficStore {
    let traffic = new TrafficStore();
    traffic.apply({
      topic: 'traffic',
      value: { type: 'snapshot', value: value ? [value] : [] },
    });
    return traffic;
  }

  class UnavailableTrafficStore extends TrafficStore {
    constructor(value: PublishedTrafficTarget) {
      super();
      this.apply({ topic: 'traffic', value: { type: 'snapshot', value: [value] } });
    }

    override subscribe(subscriber: TrafficSubscriber): () => void {
      let update: TrafficUpdate = {
        type: 'delta',
        value: { upserts: [], removed: [target.id] },
      };
      subscriber(update, new Map());
      return () => {};
    }
  }

  const instruments = createInstrumentsStore(ownship);

  const { Story } = defineMeta({
    title: 'Screens/Traffic details',
    component: TrafficDetails,
    args: {
      backLabel: 'Back',
      id: target.id,
      instruments,
      locale: 'en',
      onBack: fn(),
      traffic: createTrafficStore(target),
      units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
    },
    parameters: {
      layout: 'fullscreen',
      docs: {
        description: {
          component:
            'Traffic details show the latest target report and its relation to ownship. A stale report keeps its last values. A target that disappears from the live traffic set remains visible as unavailable. Distance and bearing stay visible as unavailable until ownship has a position.',
        },
      },
    },
  });
</script>

<Story name="Fresh target" />

<Story
  name="Stale target"
  args={{
    traffic: createTrafficStore({
      ...target,
      altitudeMslMeters: null,
      trackDegrees: null,
      alarmLevel: 'important',
      stale: true,
    }),
  }}
/>

<Story name="Unavailable target" args={{ traffic: new UnavailableTrafficStore(target) }} />

<Story name="No ownship position" args={{ instruments: createInstrumentsStore(null) }} />

<Story name="Not found" args={{ traffic: createTrafficStore(null) }} />
