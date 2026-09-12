// Golden generator for the `EllipsoidGeodesic` Vincenty fidelity fix
// (Track B3-2a0): proves the Rust port reproduces CesiumJS surface distance,
// both headings, and — crucially — the *direct* formula used by
// `interpolateUsingFraction` / `interpolateUsingSurfaceDistance`, which the port
// had replaced with a linear lon/lat lerp.
//
// Run: node specs/tests/core_fidelity/golden_ellipsoid_geodesic.mjs
//
// Follows the golden_terrain_transform.mjs pattern (direct ESM import of
// packages/engine source). Output values are inlined into
// ellipsoid_geodesic_fidelity_spec.rs as golden constants.

/* global process */

import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const dirname = path.dirname(fileURLToPath(import.meta.url));
const ENGINE = pathToFileURL(
  path.resolve(dirname, '../../../../packages/engine/Source/Core/'),
).href;

async function main() {
  const { default: EllipsoidGeodesic } = await import(`${ENGINE}/EllipsoidGeodesic.js`);
  const { default: Cartographic } = await import(`${ENGINE}/Cartographic.js`);
  const { default: Ellipsoid } = await import(`${ENGINE}/Ellipsoid.js`);
  const { default: CesiumMath } = await import(`${ENGINE}/Math.js`);

  const toRadians = CesiumMath.toRadians;

  // (name, startLonDeg, startLatDeg, endLonDeg, endLatDeg)
  const cases = [
    ['equatorial_quarter', 0.0, 0.0, 90.0, 0.0],
    ['meridional', -75.0, -40.0, -75.0, 40.0],
    ['long_haul', -75.0, 40.0, 120.0, -25.0],
    ['short_step', -100.0, 35.0, -99.0, 36.0],
    ['near_pole', 10.0, 80.0, -160.0, 78.0],
  ];

  const fractions = [0.0, 0.25, 0.5, 0.75, 1.0];

  for (const [name, slon, slat, elon, elat] of cases) {
    const start = new Cartographic(toRadians(slon), toRadians(slat), 0.0);
    const end = new Cartographic(toRadians(elon), toRadians(elat), 0.0);
    const geo = new EllipsoidGeodesic(start, end, Ellipsoid.WGS84);
    process.stdout.write(`${name}\n`);
    process.stdout.write(`  start          = ${start.longitude} ${start.latitude}\n`);
    process.stdout.write(`  end            = ${end.longitude} ${end.latitude}\n`);
    process.stdout.write(`  surfaceDistance = ${geo.surfaceDistance}\n`);
    process.stdout.write(`  startHeading   = ${geo.startHeading}\n`);
    process.stdout.write(`  endHeading     = ${geo.endHeading}\n`);
    for (const f of fractions) {
      const p = geo.interpolateUsingFraction(f);
      process.stdout.write(`  fraction ${f} = ${p.longitude} ${p.latitude} ${p.height}\n`);
    }
    // The direct formula must also accept an absolute surface distance.
    const half = geo.interpolateUsingSurfaceDistance(geo.surfaceDistance * 0.3);
    process.stdout.write(`  distance 0.3   = ${half.longitude} ${half.latitude} ${half.height}\n`);
  }

  // Coincident endpoints: `vincentyInverseFormula` short-circuits through
  // `sineSigma === 0`, so the distance collapses to 0 and the direct formula is
  // asked to walk nowhere. The third case deliberately uses a latitude outside
  // [-pi/2, pi/2] to pin down that CesiumJS does *not* normalise it: the
  // `atan((a/b) * tan(theta))` back-conversion returns the principal value.
  for (const [lon, lat] of [[1.0, 0.5], [0.3, -1.2], [1.0, 2.0]]) {
    const geo = new EllipsoidGeodesic(
      new Cartographic(lon, lat, 0.0),
      new Cartographic(lon, lat, 0.0),
      Ellipsoid.WGS84,
    );
    const p = geo.interpolateUsingFraction(0.5);
    process.stdout.write(
      `coincident ${lon} ${lat} -> ${geo.surfaceDistance} ${geo.startHeading} `
        + `${p.longitude} ${p.latitude} ${p.height}\n`,
    );
  }
}

main().catch((e) => {
  process.stderr.write(`${e}\n`);
  process.exit(1);
});
