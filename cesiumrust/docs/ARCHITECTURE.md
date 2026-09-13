# CesiumRust Architecture

> A Rust rewrite of [CesiumJS](https://cesium.com) — an open-source 3D globe and geospatial visualization engine.
>
> Built with **hexagonal architecture** (ports & adapters), **Bevy ECS** for rendering, and a commitment to **bit-exact fidelity** with CesiumJS numerical output.

---

## Table of Contents

1. [Project Overview](#project-overview)
2. [Architecture](#architecture)
3. [Layer Details](#layer-details)
4. [Key Design Decisions](#key-design-decisions)
5. [Module Reference](#module-reference)
6. [Dependency Graph](#dependency-graph)
7. [Data Flow](#data-flow)
8. [Plugin System](#plugin-system)
9. [Testing Strategy](#testing-strategy)
10. [Getting Started](#getting-started)
11. [Contributing Guide](#contributing-guide)

---

## Project Overview

CesiumRust is a ground-up Rust rewrite of CesiumJS, the industry-standard JavaScript library for 3D globes and geospatial visualization. The project aims to deliver equivalent functionality with the safety, performance, and concurrency guarantees of Rust, while maintaining **bit-exact numerical fidelity** with the original CesiumJS computation pipeline.

### Scope

- **3D Globe Rendering**: WGS84 ellipsoid with Web Mercator tile patches, dynamic LOD, and satellite imagery texturing
- **3D Tiles**: OGC 3D Tiles 1.1 (b3dm, i3dm, pnts) with hierarchical LOD, picking, and styling
- **Terrain**: Heightmap terrain with quantized-mesh decoding, LOD refinement, and imagery draping
- **Imagery Layers**: Stackable imagery layers with alpha blending and time-dynamic imagery
- **Entities API**: Billboards, polylines, polygons, 3D models, labels with time-dynamic properties
- **Camera**: Orbit, flight, and morph camera controllers with collision detection
- **Atmosphere**: Sky atmosphere, sun/stars celestial bodies, fog scattering
- **Data Sources**: GeoJSON, CZML, KML, GPX loading and visualization
- **Materials**: Fabric material system (CesiumJS-compatible), PBR, custom shaders
- **Widgets**: Animation timeline, geocoder search, scene mode picker

### Technology Stack

| Category | Technology |
|----------|-----------|
| Language | Rust 2021 edition |
| Rendering Engine | Bevy 0.15 (ECS + wgpu) |
| Math | glam 0.29 (f64, serde) |
| Async Runtime | tokio (multi-threaded) |
| HTTP Client | ureq 2.x |
| Image Decoding | image 0.25 (PNG, JPEG) |
| Serialization | serde + serde_json |
| Mesh Triangulation | earcut 0.4 |
| Error Handling | thiserror 2.x |

---

## Architecture

CesiumRust employs **hexagonal architecture** (aka ports & adapters), which separates the pure domain logic from framework and infrastructure concerns. This allows the domain to be tested in isolation, swapped between rendering backends, and evolved independently of IO dependencies.

```
┌─────────────────────────────────────────────────────────────────┐
│                        APPLICATION                               │
│          cesium-app  (Bevy App assembly, 1 crate)                │
│                                                                  │
│   • Wires adapters to domain via ports                           │
│   • Configures plugins & system ordering                         │
│   • Local modules: orbit camera, starfield, atmosphere glow      │
├─────────────────────────────────────────────────────────────────┤
│                           PORTS                                  │
│                                                                  │
│   cesium-ports-driving  (application API trait contracts)        │
│     • TileRenderer, ImageryProvider, TerrainProvider             │
│     • DataSourceLoader, EntityManager, WidgetFactory             │
│                                                                  │
│   cesium-ports-driven   (IO trait contracts)                     │
│     • GpuSink, HttpFetcher, ImageDecoder, TerrainDecoder         │
│     • GpuShader, TextureAtlas, BufferAllocator                   │
├───────────────────────┬─────────────────────────────────────────┤
│     ADAPTERS          │           ADAPTERS                       │
│                       │                                          │
│  cesium-bevy-render   │   cesium-network  (ureq HTTP adapter)   │
│  • Bevy ECS + wgpu   │   cesium-decoders (image/terrain/glTF)   │
│  • GPU mesh/texture  │                                          │
│  • Camera controllers│                                          │
│  • 13 plugins        │                                          │
├───────────────────────┴─────────────────────────────────────────┤
│                          DOMAIN                                  │
│                                                                  │
│   31 crates  ·  pure Rust  ·  framework-free  ·  f64 precision  │
│                                                                  │
│  geospatial   time     camera    event     resource    terrain   │
│  imagery      tileset  gltf      scene     datasource  material  │
│  atmosphere   interact effects   shadow    crs         kml       │
│  gpx          provider styling   globe     quadtree    primitive │
│  animation    implicit-tiling   vector    scene-mode  perform   │
│  voxel        widgets                                            │
│                                                                  │
│  Bit-exact fidelity with CesiumJS (verified against Cesium Specs)│
└─────────────────────────────────────────────────────────────────┘
```

### Hexagonal Architecture: Why It Matters

Traditional game engines and 3D viewers couple domain logic (how to compute tile visibility) with rendering infrastructure (how to submit draw calls). This coupling makes it hard to:

1. **Test domain logic** without a GPU
2. **Swap rendering backends** (e.g., Bevy → custom wgpu → headless)
3. **Reason about correctness** independent of frame timing

Hexagonal architecture inverts this: the domain defines what it *needs* via traits (ports), and adapters *implement* those traits. The application layer wires them together. The domain never imports Bevy, wgpu, ureq, or any framework.

### Layer Boundaries

#### Domain → Ports
Domain crates define trait interfaces in `ports/driving` and `ports/driven`. They depend on ports, not the reverse.

#### Ports → Adapters
Adapters implement port traits. For example, `cesium-bevy-render` implements `GpuSink` from `cesium-ports-driven`. The domain calls `GpuSink::submit_geometry()` and the adapter converts domain `GeometryData` (f64) to Bevy `Mesh` (f32) and sends it to the GPU.

#### Adapters → Application
The application crate (`cesium-app`) is the composition root. It instantiates Bevy with `DefaultPlugins`, registers all `cesium_bevy_render` plugins, and calls `App::run()`. It is the only crate with a `main()` function.

---

## Layer Details

### Domain Layer (31 crates)

The domain layer contains all pure business logic. Every crate is `no_std` compatible (though currently uses `std` for convenience during initial development) and has zero dependency on Bevy, tokio, wgpu, or any rendering framework.

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| `cesium-geospatial` | Cartesian/cartographic/geographic math, ellipsoid, bounding volumes, frustum, geometry generation | `Ellipsoid`, `Cartographic`, `Cartesian3`, `BoundingSphere`, `OrientedBoundingBox`, `GeometryData`, `Frustum` |
| `cesium-time` | Julian date, TAI, UTC, clock simulation | `JulianDate`, `Clock`, `TimeInterval` |
| `cesium-camera` | Camera model, view/projection matrices | `Camera`, `CameraFlightPath` |
| `cesium-event` | Event routing and subscription | `Event`, `EventDispatcher` |
| `cesium-resource` | Async resource loading abstraction | `Resource`, `ResourceLoader` |
| `cesium-terrain` | Terrain providers, heightmap sampling, quantized-mesh encoding | `TerrainProvider`, `TerrainMesh`, `HeightmapTerrainData` |
| `cesium-imagery` | Imagery layer stack, tile coordinate math, alpha blending | `ImageryLayer`, `ImageryProvider`, `PixelColor` |
| `cesium-tileset` | 3D Tiles 1.0/1.1, tile selection, refinement strategies | `Cesium3DTileset`, `Tile`, `TileContent` |
| `cesium-gltf` | glTF 2.0 model loading, mesh/accessor/material parsing | `GltfModel`, `GltfMesh`, `GltfMaterial` |
| `cesium-scene` | Scene graph, frustum culling, render queue | `Scene`, `CullVolume`, `DrawCommand` |
| `cesium-datasource` | GeoJSON, CZML data source abstraction | `DataSource`, `EntityCluster` |
| `cesium-material` | Material system, shader abstraction, uniform management | `Material`, `MaterialAppearance`, `UniformValue` |
| `cesium-atmosphere` | Sky atmosphere model, fog, lighting parameters | `SkyAtmosphere`, `LightingParams`, `StarSphere` |
| `cesium-interaction` | Mouse, keyboard, touch input abstraction | `ScreenSpaceEventHandler`, `CameraEventType` |
| `cesium-effects` | Post-processing effects | `PostProcessStage`, `EffectUniforms` |
| `cesium-shadow` | Shadow map generation | `ShadowMap`, `ShadowVolume` |
| `cesium-crs` | Coordinate reference system transforms | `GeographicProjection`, `WebMercatorProjection` |
| `cesium-kml` | KML document parsing | `KmlDocument`, `KmlFeature` |
| `cesium-gpx` | GPX track/route parsing | `GpxDocument`, `GpxTrack` |
| `cesium-provider` | Data provider abstraction | `Provider`, `TileAvailability` |
| `cesium-styling` | Visual styling, CSS-like property system | `Style`, `Cesium3DTileStyle` |
| `cesium-globe` | Ellipsoid globe surface, tile quadtree | `Globe`, `GlobeSurfaceTile` |
| `cesium-quadtree` | Quadtree tile subdivision and traversal | `QuadtreePrimitive`, `TileReplacementQueue` |
| `cesium-primitives` | Geometric primitives (boxes, cylinders, spheres, walls) | `BoxGeometry`, `CylinderGeometry`, `WallGeometry` |
| `cesium-animation` | Keyframe animation, interpolation | `Animation`, `Keyframe`, `InterpolationAlgorithm` |
| `cesium-implicit-tiling` | Implicit tiling (3D Tiles Next) | `ImplicitTileset`, `Subtree` |
| `cesium-vector` | Vector data rendering (polylines, polygons on terrain) | `VectorTile`, `VectorPrimitive` |
| `cesium-scene-mode` | 2D / Columbus View / 3D scene modes | `SceneMode`, `MorphTime` |
| `cesium-performance` | Performance monitoring, frame timing, metrics | `PerformanceDisplay`, `FrameRateMonitor` |
| `cesium-voxel` | Voxel rendering and data structures | `VoxelGrid`, `VoxelPrimitive` |
| `cesium-widgets` | UI widget abstractions | `Widget`, `WidgetLayout` |

### Ports Layer (2 crates)

The ports layer defines the **contract** between domain and infrastructure.

#### `cesium-ports-driving` (Application API)
Traits that the application layer uses to control domain behavior:

```rust
pub trait TileRenderer { ... }
pub trait ImageryProvider { ... }
pub trait TerrainProvider { ... }
pub trait DataSourceLoader { ... }
```

#### `cesium-ports-driven` (IO Interfaces)
Traits that adapters must implement to provide infrastructure services:

```rust
pub trait GpuSink { ... }        // Submit geometry to GPU
pub trait HttpFetcher { ... }    // Fetch remote resources
pub trait ImageDecoder { ... }   // Decode image formats
pub trait TerrainDecoder { ... } // Decode terrain formats
```

### Adapters Layer (3 crates)

#### `cesium-bevy-render`
The Bevy ECS rendering adapter — the largest adapter, comprising 13 plugins:

- Converts f64 domain `GeometryData` to f32 Bevy `Mesh` (precision boundary)
- Converts domain `TerrainMesh` to Bevy `Mesh` with imagery texturing
- Manages 3D Tiles loading, traversal, and rendering via ECS systems
- Implements camera orbit/flight controllers with screen-space input
- Renders atmosphere sky, celestial bodies
- Provides UI widgets (animation timeline, geocoder, scene mode picker)
- Debug visualization (bounding volumes, tile stats)

#### `cesium-network`
HTTP client adapter using `ureq`. Implements `HttpFetcher` to fetch tile data, terrain, imagery, and glTF models from remote servers.

#### `cesium-decoders`
Image and terrain format decoders. Implements `ImageDecoder` (PNG, JPEG via `image` crate) and `TerrainDecoder` (quantized-mesh, heightmap formats).

### Application Layer (1 crate)

#### `cesium-app`
The composition root — assembles the Bevy application:

- Instantiates `App::new()` with Bevy `DefaultPlugins`
- Registers all 13 `cesium_bevy_render` plugins
- Contains local modules for bespoke features:
  - `orbit_camera`: mouse orbit/zoom controller with grab-the-globe tracking
  - `dynamic_globe`: view-dependent LOD tile management with Bing Maps imagery
  - `atmosphere_glow`: atmospheric limb glow mesh
  - `starfield`: procedural starfield background
  - `tile_mesh`: Web Mercator tile mesh generation on the WGS84 ellipsoid
  - `tile_loader` / `bing_tile_loader`: parallel HTTP tile download and caching

---

## Key Design Decisions

### 1. Hexagonal Architecture

**Decision**: Separate domain, ports, and adapters into distinct crates with strict dependency rules.

**Rationale**:
- Domain logic (31 crates) can be tested without a GPU, network, or rendering framework
- Rendering backends can be swapped (Bevy → custom wgpu → headless) by replacing one adapter
- New IO adapters (e.g., reqwest instead of ureq) can be added without touching domain code
- Enforces discipline: domain code cannot cheat by importing platform-specific types

**Trade-off**: More crates (38 total) means more `Cargo.toml` files and compilation units. Mitigated by Rust's incremental compilation and workspace-level shared dependencies.

### 2. Bevy ECS for Rendering

**Decision**: Use Bevy 0.15 as the rendering engine, leveraging its Entity Component System.

**Rationale**:
- ECS naturally models large numbers of tiles (each tile is an entity with mesh, material, transform components)
- System-based scheduling allows clean separation of concerns (camera update, tile traversal, mesh generation, texture loading all run as separate systems)
- wgpu backend provides cross-platform GPU access (Vulkan, Metal, DX12, WebGPU)
- Scene graph, input handling, and UI are built-in

**Trade-off**: Bevy's rapid release cycle means API breakage between versions. Mitigated by pinning to 0.15 and abstracting Bevy-specific types behind adapter boundaries.

### 3. Hybrid Mode: ECS Direct for Rendering, Traits for IO

**Decision**: Render tiles as direct Bevy entities with ECS components, but use trait-based ports for network and file IO.

**Rationale**: Rendering a globe requires thousands of tile entities — ECS is the optimal pattern for this. But network requests and file decoding are better modeled as async trait implementations that the domain can call without knowing about Bevy.

### 4. f64 vs f32 Coordinate Precision

**Decision**: Use f64 throughout the domain layer, convert to f32 at the GPU boundary.

**Rationale**:
- A single-precision float (f32) has ~7 decimal digits of precision. At Earth scale (~6,371,000 meters), this gives ~0.6m resolution — insufficient for close-up terrain detail.
- Double precision (f64) provides ~15 decimal digits, sub-millimeter precision at Earth scale.
- CesiumJS uses JavaScript's native f64, so bit-exact fidelity requires matching precision.
- GPU hardware uses f32 natively; the conversion from f64 to f32 happens exactly once in `geometry_to_mesh()` at `cesium-bevy-render/src/lib.rs:79`.

### 5. Bit-Exact Fidelity with CesiumJS

**Decision**: Every numerical computation in the domain layer must produce identical output to the equivalent CesiumJS computation.

**Implementation**:
- The workspace `glam` dependency explicitly **disables** `fast-math` feature to avoid FMA (fused multiply-add) which rounds differently from CesiumJS's standard IEEE-754 two-rounding
- Integration tests in `specs/` are ported directly from CesiumJS Specs test suite
- CI verifies that all domain tests pass with bit-exact output

### 6. Plugin Pattern for Feature Modules

**Decision**: Each rendering feature (tileset, terrain, imagery, camera, atmosphere, effects, widgets) is a separate Bevy `Plugin`.

**Rationale**: Plugins are composable — users can enable only the features they need. A minimal headless render might use only `CesiumCorePlugin` + `CesiumTilesetPlugin`, while the full viewer adds all 13.

### 7. View-Dependent LOD

**Decision**: Use view-dependent level-of-detail (LOD) for tile selection, matching CesiumJS's approach.

**Implementation** (in `cesium-app/src/dynamic_globe.rs`):
- Zoom levels 3–12, with hysteresis thresholds preventing oscillation
- Only ~121 tiles visible at any time (view-dependent window)
- 8 parallel download threads for tile imagery
- Persistent texture cache (avoid re-downloading previously seen tiles)
- Progressive transition: old-zoom tiles remain visible until 20% of new-zoom tiles have textures

---

## Module Reference

### Plugin Catalog

All plugins live in `cesium-bevy-render` and are registered in `cesium-app/src/main.rs`.

| # | Plugin | Module | Purpose | Systems |
|---|--------|--------|---------|---------|
| 1 | `CesiumCorePlugin` | `lib.rs` | Initialize GlobeConfig, RenderScale, TileLoadStats, scene lighting | `setup_lighting` (Startup) |
| 2 | `CesiumCameraPlugin` | `camera/mod.rs` | Camera orbit/flight controllers, scene mode, input processing | `camera_controller_system` (PreUpdate), `camera_update_system`, `camera_flight_system`, `scene_mode_system` (PostUpdate) |
| 3 | `CesiumTilesetPlugin` | `tileset/mod.rs` | 3D Tiles (b3dm, i3dm, pnts) loading, traversal, rendering | `tile_traversal_system`, `tile_render_system`, `tile_loader_system`, `style_system` |
| 4 | `CesiumTerrainPlugin` | `terrain/mod.rs` | Heightmap terrain with LOD, quantized-mesh rendering | `terrain_lod_system`, `terrain_render_system`, `terrain_tile_loader` |
| 5 | `CesiumImageryPlugin` | `imagery/mod.rs` | Imagery layer stack, tile loading, alpha blending | `imagery_layer_manager`, `imagery_blend_system`, `imagery_tile_loader` |
| 6 | `CesiumEntityPlugin` | `entity/mod.rs` | Billboard, polyline, polygon, model, label entities with time-dynamic properties | `entity_visualizer_system`, `time_dynamic_update_system`, `billboard_face_camera_system` |
| 7 | `CesiumMaterialPlugin` | `material_system.rs` | Fabric material system (CesiumJS-compatible), material animation | (material management systems) |
| 8 | `CesiumAtmospherePlugin` | `atmosphere/mod.rs` | Sky atmosphere, sun/stars/celestial bodies rendering | `celestial_system`, `sky_system` (Update) |
| 9 | `CesiumEffectsPlugin` | `effects/mod.rs` | Post-processing effects pipeline | (post-process stage systems) |
| 10 | `CesiumWidgetPlugin` | `widgets/mod.rs` | UI widgets: animation timeline, geocoder, scene mode picker | `animation_widget_system`, `geocoder_widget_system`, `scene_mode_picker_system` (Update) |
| 11 | `DebugPlugin` | `tileset/debug_plugin.rs` | Debug visualization: bounding volumes, tile stats overlay | `debug_toggle_system`, `draw_bounding_volumes`, `update_tile_stats`, `spawn_stats_overlay` |
| 12 | `FabricMaterialPlugin` | `fabric_material.rs` | Fabric material shader management | (fabric material systems) |
| 13 | `CesiumDataSourcePlugin` | `datasource/mod.rs` | Data source loading: CZML, GeoJSON, KML, GPX | `czml_load_system`, `geojson_load_system`, `kml_load_system`, `gpx_load_system` |

### Local Modules (cesium-app only)

| Module | File | Purpose |
|--------|------|---------|
| `OrbitCameraPlugin` | `orbit_camera.rs` | Mouse orbit/zoom controller with exact grab-the-globe tracking |
| `StarfieldPlugin` | `starfield.rs` | Procedural starfield background (random point distribution on celestial sphere) |
| `AtmosphereGlowPlugin` | `atmosphere_glow.rs` | Atmospheric limb glow ring effect around the globe edge |
| `DynamicGlobePlugin` | `dynamic_globe.rs` | View-dependent LOD tile manager with Bing Maps imagery, parallel downloads, texture cache |
| `BaseSpherePlugin` | `main.rs` (inline) | Static base sphere + polar caps as safety net beneath dynamic tiles |

---

## Dependency Graph

### Crate-Level Dependencies

```
cesium-app
├── cesium-bevy-render ──────────────┐
│   ├── cesium-geospatial            │
│   ├── cesium-imagery               │
│   ├── cesium-terrain               │
│   ├── cesium-tileset               │
│   ├── cesium-quadtree              │
│   ├── cesium-scene                 │
│   ├── cesium-datasource            │
│   ├── cesium-material              │
│   ├── cesium-kml                   │
│   ├── cesium-gpx                   │
│   ├── cesium-time                  │
│   ├── cesium-animation             │
│   ├── cesium-ports-driven          │
│   ├── cesium-network               │
│   ├── cesium-gltf                  │
│   ├── cesium-camera                │
│   ├── cesium-interaction           │
│   ├── cesium-scene-mode            │
│   ├── cesium-atmosphere            │
│   ├── cesium-widgets               │
│   ├── cesium-effects               │
│   └── cesium-globe                 │
├── cesium-geospatial                │
├── cesium-time                      │
├── cesium-camera                    │
├── cesium-event                     │
├── cesium-resource                  │
├── cesium-ports-driven              │
├── cesium-ports-driving             │
├── cesium-material                  │
├── cesium-atmosphere                │
├── bevy                             │
├── glam                             │
├── ureq                             │
└── image                            │

cesium-bevy-render
├── bevy                             │
├── glam                             │
├── [21 domain crates]               │
├── cesium-network                   │
├── serde_json                       │
├── tokio                            │
└── image                            │
```

### Plugin Registration Order

Plugins in `cesium-app/src/main.rs` are registered in dependency order:

```
1. CesiumCorePlugin        (no dependencies — sets up base resources)
2. CesiumCameraPlugin      (depends on Core for GlobeConfig)
3. OrbitCameraPlugin       (independent — manages its own camera)
4. BaseSpherePlugin        (depends on Core for RenderScale)
5. DynamicGlobePlugin      (depends on Core for RenderScale)
6. CesiumTilesetPlugin     (depends on Core, Camera)
7. CesiumTerrainPlugin     (depends on Core, Camera)
8. CesiumImageryPlugin     (depends on Core, Terrain)
9. CesiumEntityPlugin      (depends on Core, Camera)
10. CesiumMaterialPlugin   (depends on Core)
11. CesiumAtmospherePlugin (depends on Core, Camera)
12. CesiumEffectsPlugin    (depends on Core)
13. AtmosphereGlowPlugin   (independent visual effect)
14. StarfieldPlugin        (independent visual effect)
15. CesiumWidgetPlugin     (depends on Core, Camera)
16. DebugPlugin            (depends on Tileset for tile stats)
```

---

## Data Flow

### Frame Rendering Pipeline

```
User Input (mouse/keyboard)
    │
    ▼
[CesiumCameraPlugin] ── reads input, updates CesiumCamera state
[OrbitCameraPlugin]  ── reads input, updates OrbitState, moves camera transform
    │
    ▼
[CesiumTilesetPlugin] ── traverses tile tree, selects visible tiles
[CesiumTerrainPlugin] ── selects terrain tiles based on camera LOD
[CesiumImageryPlugin] ── manages imagery layer visibility
    │
    ▼
[Tile loaders] ── spawn async HTTP requests (via ureq) for missing tiles
    │
    ▼
[Bevy Asset System] ── receives loaded data, creates Mesh + Image assets
    │
    ▼
[Scene pipeline] ── frustum culling → draw command generation → GPU submit
    │
    ▼
[GPU (wgpu)] ── renders to swapchain image
    │
    ▼
[Window] ── presents final frame
```

### Tile Data Flow (Bing Imagery)

```
Camera position → compute target zoom level → compute visible tiles
    │
    ▼
Start parallel downloads (8 threads)
    │
    ├─ Tile (0,0,3) → Bing Maps URL → HTTP GET → JPEG decode → RGBA bytes
    ├─ Tile (1,0,3) → Bing Maps URL → HTTP GET → JPEG decode → RGBA bytes
    └─ ...
    │
    ▼
Apply textures via Bevy material system
    │
    ▼
Progressive cleanup: remove old-zoom tiles when new tiles are textured
```

---

## Plugin System

### How Plugins Work in CesiumRust

Each plugin in `cesium-bevy-render` follows a consistent pattern:

1. **Module structure**: `src/<feature>/mod.rs` with sub-modules for systems, components, loaders
2. **Plugin struct**: Implements `bevy::app::Plugin` with a `build()` method
3. **Resource registration**: `init_resource::<T>()` for plugin-specific state
4. **System registration**: `add_systems(Update, ...)` for per-frame processing
5. **Component registration**: via `app.register_type::<T>()` for editor reflection

### Adding a New Plugin

To add a new rendering feature:

1. Create domain crate in `domain/<feature>/` with pure logic
2. Define port traits in `ports/driving/` or `ports/driven/`
3. Create adapter module in `adapters/bevy-render/src/<feature>/`
4. Implement `Plugin` trait, registering resources and systems
5. Re-export from `adapters/bevy-render/src/lib.rs`
6. Register in `application/cesium-app/src/main.rs`

---

## Testing Strategy

### Unit Tests (Domain Layer)

Domain crates have unit tests that verify numerical correctness. These run without a GPU or Bevy:

```bash
cargo test -p cesium-geospatial
cargo test -p cesium-time
cargo test -p cesium-camera
# ... all 31 domain crates
```

Key domain tests verify:
- Ellipsoid surface normal computation
- Cartographic ↔ Cartesian coordinate conversion
- Julian date arithmetic
- Frustum culling mathematics
- Tile quadtree traversal
- Quadtree add/remove operations

### Integration Tests (specs/)

The `specs/` crate contains tests ported from CesiumJS Specs:

```bash
cargo test -p specs
```

These tests verify bit-exact fidelity:
- Same input → same output as CesiumJS
- No floating-point drift from FMA or rounding differences
- Validated against CesiumJS CI golden data

### Adapter Tests (bevy-render)

Tests in `adapters/bevy-render/src/lib.rs` verify the f64→f32 conversion boundary:

- `test_geometry_to_mesh`: geometry conversion preserves vertex attributes
- `test_create_ellipsoid_mesh`: ellipsoid mesh has correct topology
- `test_terrain_mesh_to_bevy`: terrain mesh conversion
- `test_create_imagery_texture`: texture creation from raw RGBA
- `test_create_solid_color_texture`: fallback texture generation

```bash
cargo test -p cesium-bevy-render
```

### Running All Tests

```bash
cargo test --workspace
```

---

## Getting Started

### Prerequisites

- **Rust** 1.80+ (install via [rustup](https://rustup.rs))
- **System libraries** for Bevy (Ubuntu):
  ```bash
  sudo apt install libudev-dev libasound2-dev libxkbcommon-dev vulkan-validationlayers
  ```

### Build & Run

```bash
# Clone
git clone <repo-url> cesiumrust
cd cesiumrust

# Build (debug)
cargo build

# Run the 3D globe viewer
cargo run

# Run with release optimizations
cargo run --release

# Check compilation without producing a binary
cargo check
```

### Controls

| Action | Input |
|--------|-------|
| Orbit / rotate globe | Left mouse drag |
| Zoom in/out | Mouse wheel |
| Toggle bounding volumes | F1 |
| Toggle tile stats | F2 |
| Toggle LOD debug | F3 |

### Expected Behavior

On first run, the viewer displays:
- A dark blue sphere (base safety net) with polar ice caps
- Dynamic tile grid that loads Bing Maps satellite imagery (requires internet)
- Starfield background
- Atmospheric limb glow around the globe edge
- Widget UI (animation timeline, geocoder)

Some plugins may log warnings if no data sources are configured (e.g., no 3D Tiles URL, no terrain provider). This is normal.

---

## Contributing Guide

### Development Workflow

1. **Fork & clone** the repository
2. **Create a branch**: `git checkout -b feature/my-feature`
3. **Write domain logic**: Start in the appropriate domain crate. No Bevy imports allowed here.
4. **Define ports**: Add trait methods to `ports/driving` or `ports/driven` as needed.
5. **Implement adapter**: Add Bevy systems/components in `adapters/bevy-render`.
6. **Wire in cesium-app**: Register the new plugin in `src/main.rs`.
7. **Add tests**: Unit tests in domain, integration tests in `specs/`.
8. **Run full test suite**: `cargo test --workspace`
9. **Check formatting**: `cargo fmt --all -- --check`
10. **Check lints**: `cargo clippy --workspace -- -D warnings`

### Code Style

- Follow Rust 2021 edition conventions
- Use `rustfmt` for formatting (no custom config)
- Use `clippy` for linting with `-D warnings`
- Domain crates: `#![deny(unsafe_code)]`
- Adapter crates: `unsafe` allowed only for FFI and GPU interaction, documented with `// SAFETY:` comments
- Naming: match CesiumJS conventions where applicable (e.g., `Cartesian3`, `Cartographic`, `JulianDate`)

### Commit Conventions

- `feat:` new feature
- `fix:` bug fix
- `domain:` domain layer changes
- `adapter:` adapter layer changes
- `app:` application layer changes
- `docs:` documentation only
- `test:` test additions or fixes
- `refactor:` code restructuring without behavior changes

### Adding a New Domain Crate

```bash
# 1. Create the crate
mkdir -p domain/new-feature/src
cp domain/geospatial/Cargo.toml domain/new-feature/Cargo.toml
# Edit name, description

# 2. Add to workspace
# Edit /Cargo.toml: add "domain/new-feature" to members
# Edit /Cargo.toml: add workspace dependency entry

# 3. Write code
# domain/new-feature/src/lib.rs

# 4. Add tests
# domain/new-feature/src/lib.rs (inline #[cfg(test)])

# 5. Verify
cargo test -p cesium-new-feature
```

### Architecture Rules (Enforced by Review)

1. **Domain crates must not depend on Bevy, tokio, ureq, wgpu, or any framework.**
2. **Ports crates must contain only traits (no implementation).**
3. **Adapters must implement port traits, not be called directly from domain.**
4. **Application crate is the only crate with a `main()` function.**
5. **f64 precision is mandatory** in domain; f32 conversion happens ONLY in `geometry_to_mesh()`.
6. **No panicking** in domain code — use `Result` or `Option`.

### Getting Help

- Architecture questions: see this document
- Module-specific docs: see `domain/<crate>/README.md` or module doc comments
- CesiumJS reference: see [CesiumJS documentation](https://cesium.com/learn/cesiumjs-learn/)
- Build issues: check `rustup update` and system dependencies
