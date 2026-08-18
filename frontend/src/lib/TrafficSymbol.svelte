<script lang="ts">
  import type { TrafficAlarmLevel } from '$lib/protocol/generated/TrafficAlarmLevel';
  import type { TrafficType } from '$lib/protocol/generated/TrafficType';

  import aircraft from '../../../libs/updraft_sprites/sprites/aircraft.svg?url';
  import airship from '../../../libs/updraft_sprites/sprites/airship.svg?url';
  import balloon from '../../../libs/updraft_sprites/sprites/balloon.svg?url';
  import glider from '../../../libs/updraft_sprites/sprites/glider.svg?url';
  import hangGlider from '../../../libs/updraft_sprites/sprites/hang-glider.svg?url';
  import helicopter from '../../../libs/updraft_sprites/sprites/helicopter.svg?url';
  import jet from '../../../libs/updraft_sprites/sprites/jet.svg?url';
  import paraglider from '../../../libs/updraft_sprites/sprites/paraglider.svg?url';
  import unknown from '../../../libs/updraft_sprites/sprites/unknown.svg?url';

  type Props = {
    trafficType: TrafficType;
    trackDegrees?: number | null;
    alarmLevel?: TrafficAlarmLevel;
    stale?: boolean;
    class?: string;
  };

  const TRAFFIC_SYMBOLS = {
    unknown,
    glider,
    towPlane: aircraft,
    helicopter,
    skydiver: unknown,
    dropPlane: aircraft,
    hangGlider,
    paraglider,
    pistonAircraft: aircraft,
    jetAircraft: jet,
    balloon,
    airship,
    uav: unknown,
    staticObstacle: unknown,
  } satisfies Record<TrafficType, string>;

  let {
    trafficType,
    trackDegrees = null,
    alarmLevel = 'none',
    stale = false,
    class: className,
  }: Props = $props();

  let rotation = $derived(
    trafficType !== 'balloon' && trackDegrees !== null ? `rotate(${trackDegrees}deg)` : undefined,
  );
</script>

<span
  aria-hidden="true"
  class={['traffic-symbol', alarmLevel, { stale }, className]}
  style:--traffic-symbol-image={`url("${TRAFFIC_SYMBOLS[trafficType]}")`}
  style:transform={rotation}
></span>

<style>
  .traffic-symbol {
    display: block;
    flex: 0 0 auto;
    inline-size: var(--traffic-symbol-size, 1em);
    block-size: var(--traffic-symbol-size, 1em);
    background: currentcolor;
    -webkit-mask: var(--traffic-symbol-image) center / contain no-repeat;
    mask: var(--traffic-symbol-image) center / contain no-repeat;
    transform-origin: center;
  }

  .unknown,
  .none {
    color: var(--color-text);
  }

  .low {
    color: var(--color-amber-500);
  }

  .important {
    color: var(--color-orange-500);
  }

  .urgent {
    color: var(--color-red-600);
  }

  .stale {
    opacity: 0.45;
  }
</style>
