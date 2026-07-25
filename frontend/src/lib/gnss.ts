export type Availability<T> =
  { status: 'unavailable' } | { status: 'current'; value: T } | { status: 'lastKnown'; value: T };

/** A geographic latitude and longitude in degrees. */
export type LatLon = { latitudeDegrees: number; longitudeDegrees: number };

/** Selected GNSS components. */
export type GnssData = {
  position: Availability<LatLon>;
  altitudeMeters: Availability<number>;
  trackDegrees: Availability<number>;
  groundSpeedMetersPerSecond: Availability<number>;
};
