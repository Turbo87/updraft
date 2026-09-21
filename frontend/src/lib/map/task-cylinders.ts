// The display uses a spherical approximation. Crossing detection uses WGS84 in the core.
export function taskCylinder([longitude, latitude]: number[]): number[][] {
  let radians = Math.PI / 180;
  let lat = latitude * radians;
  let radius = 500 / 6371008.8;
  let ring = Array.from({ length: 64 }, (_, index) => {
    let bearing = (index * 2 * Math.PI) / 64;
    let targetLatitude = Math.asin(
      Math.sin(lat) * Math.cos(radius) + Math.cos(lat) * Math.sin(radius) * Math.cos(bearing),
    );
    let longitudeOffset = Math.atan2(
      Math.sin(bearing) * Math.sin(radius) * Math.cos(lat),
      Math.cos(radius) - Math.sin(lat) * Math.sin(targetLatitude),
    );
    return [longitude + longitudeOffset / radians, targetLatitude / radians];
  });
  ring.push(ring[0]);
  return ring;
}
