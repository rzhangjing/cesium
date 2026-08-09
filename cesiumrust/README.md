# CesiumRust

A Rust rewrite of [CesiumJS](https://cesium.com) — an open-source 3D globe and geospatial visualization engine.

Uses **hexagonal architecture** (ports & adapters) to separate pure domain logic from framework and IO concerns. The rendering layer is built on [Bevy](https://bevyengine.org), a data-driven game engine for Rust.

> **Architecture deep-dive:** See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for a comprehensive 3000+ word architecture document covering design decisions, module reference, dependency graph, data flow, and contributing guide.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  APPLICATION                         │
│         cesium-app (Bevy App assembly)               │
├─────────────────────────────────────────────────────┤
│                     PORTS                            │
│   driving (app APIs)    ·    driven (IO contracts)   │
├──────────────────────┬──────────────────────────────┤
│      ADAPTERS        │           ADAPTERS            │
│   bevy-render        │   network · decoders          │
├──────────────────────┴──────────────────────────────┤
│                     DOMAIN                           │
│   31 crates  ·  pure Rust  ·  framework-free         │
│   Bit-exact fidelity with CesiumJS                   │
└─────────────────────────────────────────────────────┘
```

| Layer | Crates | Description |
|-------|--------|-------------|
| **Domain** | 31 | Pure Rust logic — geometry, time, camera, terrain, imagery, tilesets, glTF, scene graph, materials, atmosphere, CRS, KML, GPX, quadtree, vector data, etc. No framework dependency. |
| **Ports** | 2 | Trait contracts. `driving` defines application-facing APIs; `driven` defines IO interfaces that adapters implement. |
| **Adapters** | 3 | `bevy-render` (Bevy ECS + wgpu rendering), `network` (ureq HTTP client), `decoders` (image/terrain decoding). |
| **Application** | 1 | `cesium-app` — assembles the Bevy app, wires adapters to domain, launches the interactive 3D globe viewer. |

## Plugins

The `cesium-app` integrates 13 plugins from `cesium-bevy-render`, plus local modules:

### Bevy Render Plugins

| Plugin | Purpose |
|--------|---------|
| `CesiumCorePlugin` | Initialize GlobeConfig, RenderScale, scene lighting |
| `CesiumCameraPlugin` | Camera orbit/flight controllers, scene mode switching |
| `CesiumTilesetPlugin` | 3D Tiles (b3dm, i3dm, pnts) loading & rendering |
| `CesiumTerrainPlugin` | Heightmap terrain with LOD refinement |
| `CesiumImageryPlugin` | Imagery layer stack with alpha blending |
| `CesiumEntityPlugin` | Billboards, polylines, polygons, models, labels |
| `CesiumMaterialPlugin` | Fabric material system (CesiumJS-compatible) |
| `CesiumAtmospherePlugin` | Sky atmosphere, celestial bodies |
| `CesiumEffectsPlugin` | Post-processing effects pipeline |
| `CesiumWidgetPlugin` | Animation timeline, geocoder, scene mode picker |
| `DebugPlugin` | Bounding volume wireframes, tile stats (F1-F3 toggles) |
| `FabricMaterialPlugin` | Fabric shader material management |
| `CesiumDataSourcePlugin` | CZML, GeoJSON, KML, GPX data source loading |

### Local Plugins

| Plugin | Purpose |
|--------|---------|
| `OrbitCameraPlugin` | Mouse orbit/zoom with grab-the-globe tracking |
| `StarfieldPlugin` | Procedural starfield background |
| `AtmosphereGlowPlugin` | Atmospheric limb glow around globe edge |
| `DynamicGlobePlugin` | View-dependent LOD tiles + Bing Maps imagery |
| `BaseSpherePlugin` | Static base sphere + polar caps safety net |

## Getting Started

```bash
# Run the interactive 3D globe viewer
cargo run

# Run the full test suite
cargo test --workspace

# Check compilation (fast, no binary)
cargo check
```

**Prerequisites:** Rust 1.80+, system dependencies for Bevy (libudev, alsa, vulkan). On Ubuntu:

```bash
sudo apt install libudev-dev libasound2-dev libxkbcommon-dev vulkan-validationlayers
```

## Controls

| Action | Input |
|--------|-------|
| Orbit / rotate globe | Left mouse drag |
| Zoom in/out | Mouse wheel |
| Toggle bounding volumes | F1 |
| Toggle tile stats overlay | F2 |
| Toggle LOD debug | F3 |

## Project Structure

```
cesiumrust/
├── Cargo.toml                 # Workspace root (38 crates)
├── domain/                    # Domain layer — 31 crates
│   ├── geospatial/            # Cartesian, cartographic, geographic math (f64)
│   ├── time/                  # Julian date, TAI, UTC
│   ├── camera/                # Camera controllers, frustum culling
│   ├── event/                 # Event system
│   ├── resource/              # Async resource loading
│   ├── terrain/               # Terrain providers, heightmap sampling
│   ├── imagery/               # Imagery layer management
│   ├── tileset/               # 3D Tiles, tile selection
│   ├── gltf/                  # glTF model loading
│   ├── scene/                 # Scene graph, culling, rendering primitives
│   ├── datasource/            # GeoJSON, CZML, KML data sources
│   ├── material/              # Material system, shader abstractions
│   ├── atmosphere/            # Sky atmosphere, fog
│   ├── interaction/           # Mouse/keyboard/touch input
│   ├── effects/               # Post-processing effects
│   ├── shadow/                # Shadow maps
│   ├── crs/                   # Coordinate reference systems
│   ├── kml/                   # KML parsing
│   ├── gpx/                   # GPX parsing
│   ├── provider/              # Data provider abstraction
│   ├── styling/               # Visual styling, CSS-like properties
│   ├── globe/                 # Ellipsoid globe surface
│   ├── quadtree/              # Quadtree tile subdivision
│   ├── primitives/            # Geometric primitives
│   ├── animation/             # Keyframe animation
│   ├── implicit-tiling/       # Implicit tiling (3D Tiles Next)
│   ├── vector/                # Vector data rendering
│   ├── scene-mode/            # 2D / 2.5D / 3D scene modes
│   ├── performance/           # Performance monitoring, metrics
│   ├── voxel/                 # Voxel rendering
│   └── widgets/               # UI widgets
├── ports/
│   ├── driving/               # Application API trait contracts
│   └── driven/                # IO trait contracts (renderer, network, loader)
├── adapters/
│   ├── bevy-render/           # Bevy ECS + wgpu rendering adapter
│   ├── decoders/              # Image, terrain, glTF decoders
│   └── network/               # HTTP client (ureq) adapter
├── application/
│   └── cesium-app/            # Bevy App — main binary entry point
├── docs/
│   └── ARCHITECTURE.md        # Comprehensive architecture document
└── specs/                     # Integration tests ported from CesiumJS Specs
```

## Verification

The domain layer maintains **bit-exact fidelity** with CesiumJS. Integration tests in `specs/` are ported directly from the CesiumJS Specs test suite and verify identical numerical output at every step.

Key design decisions enabling bit-exact fidelity:
- **f64 precision** throughout the domain layer (f32 only at GPU boundary)
- **Standard IEEE-754 rounding** (no FMA/fast-math)
- **Matching algorithm implementations** (same math, same order of operations)

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Hexagonal architecture | Test domain logic without GPU; swap rendering backends |
| Bevy ECS for rendering | Natural fit for thousands of tile entities |
| f64 domain / f32 GPU | Sub-mm precision at Earth scale vs GPU hardware limits |
| Hybrid ECS + traits | ECS for tile rendering, traits for IO decoupling |
| Bit-exact CesiumJS fidelity | Domain tests ported from CesiumJS Specs |
