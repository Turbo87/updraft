export type AltitudeUnit = 'ft' | 'm';
export type DistanceUnit = 'km' | 'mi' | 'nm';
export type SpeedUnit = 'km/h' | 'kt' | 'mph';
export type VerticalSpeedUnit = 'ft/min' | 'kt' | 'm/s';

const metersPerFoot = 0.3048;
const metersPerMile = 1_609.344;
const metersPerNauticalMile = 1_852;
const secondsPerHour = 3_600;
const secondsPerMinute = 60;

export function convertAltitude(meters: number, unit: AltitudeUnit): number {
  switch (unit) {
    case 'm':
      return meters;
    case 'ft':
      return meters / metersPerFoot;
  }
}

export function convertDistance(meters: number, unit: DistanceUnit): number {
  switch (unit) {
    case 'km':
      return meters / 1_000;
    case 'mi':
      return meters / metersPerMile;
    case 'nm':
      return meters / metersPerNauticalMile;
  }
}

export function convertSpeed(metersPerSecond: number, unit: SpeedUnit): number {
  switch (unit) {
    case 'km/h':
      return (metersPerSecond * secondsPerHour) / 1_000;
    case 'mph':
      return (metersPerSecond * secondsPerHour) / metersPerMile;
    case 'kt':
      return (metersPerSecond * secondsPerHour) / metersPerNauticalMile;
  }
}

export function convertVerticalSpeed(metersPerSecond: number, unit: VerticalSpeedUnit): number {
  switch (unit) {
    case 'm/s':
      return metersPerSecond;
    case 'ft/min':
      return (metersPerSecond * secondsPerMinute) / metersPerFoot;
    case 'kt':
      return (metersPerSecond * secondsPerHour) / metersPerNauticalMile;
  }
}
