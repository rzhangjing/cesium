// Golden generator for the CorridorOutlineGeometry combine first-corner
// index differential (CZ-01 residual: wallIndices first corner ±1).
//
// Run: node specs/tests/core_geometry/golden_corridor_outline.mjs
//
// Follows the audit/diff_golden.mjs pattern (direct ESM import of
// packages/engine source) but lives outside audit/ so the audit tooling
// is untouched. Output values are inlined into
// corridor_outline_geometry_spec.rs as golden constants.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ENGINE = pathToFileURL(
  path.resolve(__dirname, '../../../../packages/engine/Source/Core/'),
).href + '/';

function enc(v) {
  if (typeof v === 'number') {
    if (Object.is(v, -0)) return '-0';
    return v;
  }
  if (v == null || typeof v !== 'object') return v;
  if (Array.isArray(v)) return Array.from(v, enc);
  if (ArrayBuffer.isView(v)) return Array.from(v, enc);
  const o = {};
  for (const k of Object.keys(v)) o[k] = enc(v[k]);
  return o;
}

async function main() {
  const { default: CorridorOutlineGeometry } = await import(ENGINE + 'CorridorOutlineGeometry.js');
  const { default: Cartesian3 } = await import(ENGINE + 'Cartesian3.js');
  const { default: CornerType } = await import(ENGINE + 'CornerType.js');

  const cases = {};

  function run(name, options) {
    const g = new CorridorOutlineGeometry(options);
    const geo = CorridorOutlineGeometry.createGeometry(g);
    cases[name] = {
      positionCount: geo.attributes.position.values.length / 3,
      positionsHead: enc(Array.from(geo.attributes.position.values.slice(0, 12))),
      indices: enc(geo.indices),
      boundingSphere: [
        geo.boundingSphere.center.x,
        geo.boundingSphere.center.y,
        geo.boundingSphere.center.z,
        geo.boundingSphere.radius,
      ],
    };
  }

  // Case A: single corner (left turn), BEVELED, extruded — exercises
  // wallIndices incl. the first-corner push and the BEVELED extra push.
  run('beveled_extruded_one_corner', {
    positions: [
      Cartesian3.fromDegrees(0.0, 0.0),
      Cartesian3.fromDegrees(2.0, 0.0),
      Cartesian3.fromDegrees(2.0, 2.0),
    ],
    width: 100000.0,
    cornerType: CornerType.BEVELED,
    height: 100.0,
    extrudedHeight: 0.0,
  });

  // Case B: two corners (left + right), MITERED, extruded — checks that
  // subsequent corners match.
  run('mitered_extruded_two_corners', {
    positions: [
      Cartesian3.fromDegrees(0.0, 0.0),
      Cartesian3.fromDegrees(2.0, 0.0),
      Cartesian3.fromDegrees(2.0, 2.0),
      Cartesian3.fromDegrees(4.0, 2.0),
    ],
    width: 100000.0,
    cornerType: CornerType.MITERED,
    height: 200.0,
    extrudedHeight: 0.0,
  });

  // Case C: rounded, non-extruded sanity (combine without wallIndices use).
  run('rounded_flat_one_corner', {
    positions: [
      Cartesian3.fromDegrees(0.0, 0.0),
      Cartesian3.fromDegrees(2.0, 0.0),
      Cartesian3.fromDegrees(2.0, 2.0),
    ],
    width: 100000.0,
    cornerType: CornerType.ROUNDED,
    height: 0.0,
  });

  const out = JSON.stringify(cases, null, 1);
  fs.writeFileSync(path.join(__dirname, 'golden_corridor_outline.json'), out + '\n');
  console.log('written golden_corridor_outline.json (' + Object.keys(cases).length + ' cases)');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
