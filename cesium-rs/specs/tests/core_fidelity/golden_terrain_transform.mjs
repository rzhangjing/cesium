// Golden generator for the `TerrainMesh.computeTransform` z-scale fix-up
// differential (Track B3-1): proves CesiumJS produces the *same* NaN third
// column as the Rust port when the OBB's z half-axis is exactly zero, and that
// the fix-up never fires on a realistic flat tile.
//
// Run: node specs/tests/core_fidelity/golden_terrain_transform.mjs
//
// Follows the golden_corridor_outline.mjs pattern (direct ESM import of
// packages/engine source). Output values are inlined into
// terrain_pick_fidelity_spec.rs as golden constants.

/* global process */

import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const dirname = path.dirname(fileURLToPath(import.meta.url));
const ENGINE = pathToFileURL(
  path.resolve(dirname, '../../../../packages/engine/Source/Core/'),
).href;

async function main() {
  const { default: OrientedBoundingBox } = await import(`${ENGINE}/OrientedBoundingBox.js`);
  const { default: Matrix4 } = await import(`${ENGINE}/Matrix4.js`);
  const { default: Cartesian3 } = await import(`${ENGINE}/Cartesian3.js`);
  const { default: Rectangle } = await import(`${ENGINE}/Rectangle.js`);
  const { default: Ellipsoid } = await import(`${ENGINE}/Ellipsoid.js`);
  const { default: CesiumMath } = await import(`${ENGINE}/Math.js`);

  // Mirrors TerrainMesh.js#computeTransform's tail (L239-244): build the
  // transform from the OBB, then patch a degenerate z-scale by *re*-scaling
  // with setScale — which divides by the scale it just read back.
  function computeTransform(obb) {
    const result = new Matrix4();
    OrientedBoundingBox.computeTransformation(obb, result);
    const zScale = Matrix4.getScale(result, new Cartesian3()).z;
    if (zScale <= CesiumMath.EPSILON16) {
      const scale = Matrix4.getScale(result, new Cartesian3());
      scale.z = 1.0;
      Matrix4.setScale(result, scale, result);
    }
    return result;
  }

  // (name, rectangle, minimumHeight, maximumHeight)
  const cases = [
    // A realistic flat tile. `fromRectangle` measures `minZ` from the west
    // corners against the tangent plane at the rectangle's centre while `maxZ`
    // is `maximumHeight` verbatim, so the ellipsoid's sag below that plane
    // leaves a healthy z extent even though the height range is zero.
    ['realistic_flat_tile', [-0.02, -0.01, 0.02, 0.01], 250.0, 250.0],
    // The pathological input: a few nanoradians wide, so the sag underflows and
    // halfAxes column 2 is exactly the zero vector.
    ['degenerate_tiny_tile', [-1e-9, -1e-9, 1e-9, 1e-9], 500.0, 500.0],
  ];

  for (const [name, rect, minH, maxH] of cases) {
    const obb = OrientedBoundingBox.fromRectangle(
      new Rectangle(rect[0], rect[1], rect[2], rect[3]),
      minH,
      maxH,
      Ellipsoid.WGS84,
    );
    const transform = computeTransform(obb);
    const scale = Matrix4.getScale(transform, new Cartesian3());
    process.stdout.write(`${name}\n`);
    process.stdout.write(`  rectangle      = ${rect}\n`);
    process.stdout.write(`  heights        = ${minH} .. ${maxH}\n`);
    process.stdout.write(`  halfAxes       = ${Array.from(obb.halfAxes)}\n`);
    process.stdout.write(`  obb.center     = ${obb.center.x} ${obb.center.y} ${obb.center.z}\n`);
    process.stdout.write(`  transform      = ${Array.from(transform)}\n`);
    process.stdout.write(`  scale          = ${scale.x} ${scale.y} ${scale.z}\n`);
  }
}

main().catch((e) => {
  process.stderr.write(`${e}\n`);
  process.exit(1);
});
