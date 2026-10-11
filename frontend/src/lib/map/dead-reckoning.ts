import type { GpsInstruments } from '#lib/protocol/generated/GpsInstruments.js';
import type { LatLon } from '#lib/protocol/generated/LatLon.js';

const EARTH_RADIUS_METERS = 6_371_008.8;

/** Dead reckoning stops this long after the receive time of a position. */
export const DEAD_RECKONING_LIMIT_SECONDS = 1.5;

/** Gliders in thermals turn at about 15 to 25 °/s. */
const TURN_RATE_LIMIT_DEGREES_PER_SECOND = 30;

/** GPS tracks below this ground speed are noise. */
const MIN_SPEED_METERS_PER_SECOND = 2;

/** A constant ground speed and turn rate from a received position. */
export type GroundMotion = {
  speedMetersPerSecond: number;
  trackDegrees: number;
  /** Clockwise positive. */
  turnRateDegreesPerSecond: number;
};

/** The motion for dead reckoning, or `null` when the symbol must stay at its position. */
export function groundMotion(
  gps: Pick<
    GpsInstruments,
    'groundSpeedMetersPerSecond' | 'trackDegrees' | 'turnRateDegreesPerSecond' | 'stale'
  >,
): GroundMotion | null {
  let { groundSpeedMetersPerSecond, trackDegrees, turnRateDegreesPerSecond, stale } = gps;
  if (stale || groundSpeedMetersPerSecond === null || trackDegrees === null) return null;
  if (groundSpeedMetersPerSecond < MIN_SPEED_METERS_PER_SECOND) return null;
  return {
    speedMetersPerSecond: groundSpeedMetersPerSecond,
    trackDegrees,
    turnRateDegreesPerSecond: turnRateDegreesPerSecond ?? 0,
  };
}

/**
 * The position and track after `elapsedSeconds` along a circular arc.
 *
 * The arc keeps the speed and the limited turn rate constant. A flat local
 * approximation of the Earth is accurate enough for the short distance.
 */
export function deadReckon(
  position: LatLon,
  motion: GroundMotion,
  elapsedSeconds: number,
): { position: LatLon; trackDegrees: number } {
  let seconds = Math.min(Math.max(elapsedSeconds, 0), DEAD_RECKONING_LIMIT_SECONDS);
  let turnRate = radians(
    Math.min(
      Math.max(motion.turnRateDegreesPerSecond, -TURN_RATE_LIMIT_DEGREES_PER_SECOND),
      TURN_RATE_LIMIT_DEGREES_PER_SECOND,
    ),
  );
  let start = radians(motion.trackDegrees);
  let end = start + turnRate * seconds;
  let east: number;
  let north: number;
  if (turnRate === 0) {
    let distance = motion.speedMetersPerSecond * seconds;
    east = distance * Math.sin(start);
    north = distance * Math.cos(start);
  } else {
    let radius = motion.speedMetersPerSecond / turnRate;
    east = radius * (Math.cos(start) - Math.cos(end));
    north = radius * (Math.sin(end) - Math.sin(start));
  }

  let latitude = radians(position.latitudeDegrees);
  return {
    position: {
      latitudeDegrees: position.latitudeDegrees + degrees(north / EARTH_RADIUS_METERS),
      longitudeDegrees:
        position.longitudeDegrees + degrees(east / (EARTH_RADIUS_METERS * Math.cos(latitude))),
    },
    trackDegrees: degrees(end),
  };
}

function radians(degrees: number): number {
  return (degrees * Math.PI) / 180;
}

function degrees(radians: number): number {
  return (radians * 180) / Math.PI;
}
