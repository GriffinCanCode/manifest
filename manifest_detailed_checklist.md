# Grand Strategy Game - Detailed Implementation Checklist

## JavaScript/TypeScript Framework Stack

### Core Dependencies
```json
{
  "dependencies": {
    // Core Framework
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-fs": "^2.0.0",
    "@tauri-apps/plugin-dialog": "^2.0.0",
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    
    // 3D Rendering & Graphics
    "three": "^0.160.0",
    "@react-three/fiber": "^8.15.0",
    "@react-three/drei": "^9.96.0",
    "@react-three/postprocessing": "^2.16.0",
    
    // Animation
    "framer-motion": "^11.0.0",
    "lottie-react": "^2.4.0",
    "@use-gesture/react": "^10.3.0",
    
    // State Management
    "zustand": "^4.5.0",
    "immer": "^10.0.3",
    "valtio": "^1.13.0",
    
    // UI Components
    "@radix-ui/react-dialog": "^1.0.5",
    "@radix-ui/react-dropdown-menu": "^2.0.6",
    "@radix-ui/react-tabs": "^1.0.4",
    "@radix-ui/react-tooltip": "^1.0.7",
    "@tanstack/react-table": "^8.11.0",
    "@tanstack/react-virtual": "^3.0.0",
    "react-hotkeys-hook": "^4.4.0",
    
    // Data Visualization
    "d3": "^7.8.0",
    "visx": "^3.10.0",
    "recharts": "^2.12.0",
    "react-force-graph-3d": "^1.24.0",
    "cytoscape": "^3.28.0",
    "react-cytoscapejs": "^2.0.0",
    
    // Utilities
    "msgpackr": "^1.10.0",
    "comlink": "^4.4.1",
    "dexie": "^3.2.0",
    "i18next": "^23.7.0",
    "react-i18next": "^14.0.0",
    "date-fns": "^3.3.0",
    "lodash-es": "^4.17.0",
    
    // Audio
    "howler": "^2.2.0",
    "tone": "^14.7.0",
    
    // Development & Testing
    "zod": "^3.22.0",
    "@tanstack/react-query": "^5.18.0"
  },
  "devDependencies": {
    "typescript": "^5.3.0",
    "vite": "^5.0.0",
    "@vitejs/plugin-react-swc": "^3.5.0",
    "tailwindcss": "^3.4.0",
    "vitest": "^1.2.0",
    "@testing-library/react": "^14.1.0",
    "@types/three": "^0.160.0"
  }
}
```

## Rust Dependencies (Cargo.toml)

```toml
[dependencies]
# Core Framework
tauri = { version = "2.0", features = ["macos-private-api"] }
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# ECS & Game Engine
bevy_ecs = "0.12"
legion = "0.4"
hecs = "0.10"
specs = "0.20"

# Serialization & Data
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = "1.3"
rmp-serde = "1.1"  # MessagePack
postcard = "1.0"   # Efficient binary serialization

# Database & Storage
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "sqlite"] }
rocksdb = "0.21"
sled = "0.34"
redb = "1.5"

# Math & Geometry
nalgebra = "0.32"
cgmath = "0.18"
glam = "0.25"  # Fast math by Embark
euclid = "0.22"
parry2d = "0.13"  # 2D collision detection

# Procedural Generation
noise = "0.8"
bracket-noise = "0.8"
simdnoise = "3.1"
fast_poisson = "0.5"

# Pathfinding & Graphs
pathfinding = "4.8"
petgraph = "0.6"
hierarchical_pathfinding = "0.5"

# Parallel Processing
rayon = "1.8"
crossbeam = "0.8"
parking_lot = "0.12"
dashmap = "5.5"

# Compression
zstd = "0.13"
lz4_flex = "0.11"
snap = "1.1"

# Lua Scripting
mlua = { version = "0.9", features = ["lua54", "vendored", "async", "send"] }
rlua = "0.19"

# Memory Management
bumpalo = "3.14"
typed-arena = "2.0"
slotmap = "1.0"

# Networking (for future multiplayer)
quinn = "0.10"
laminar = "0.5"

# Profiling & Debug
tracy-client = "0.16"
puffin = "0.18"
criterion = "0.5"
tracing = "0.1"
tracing-subscriber = "0.3"

# AI & Decision Making
behavior-tree = "0.3"
utility-ai = "0.2"

# Random Number Generation
rand = "0.8"
rand_chacha = "0.3"
rand_xoshiro = "0.6"
turborand = "0.10"  # Fastest RNG

# Spatial Indexing
rstar = "0.11"  # R-tree
kdtree = "0.7"
spatial-join = "0.2"

# Time & Scheduling
chrono = "0.4"
cron = "0.12"

# Testing
proptest = "1.4"
quickcheck = "1.0"
rstest = "0.18"

[dev-dependencies]
cargo-watch = "8.5"
cargo-expand = "1.0"
cargo-flamegraph = "0.6"
```

## Zig Dependencies (build.zig.zon)

```zig
.{
    .name = "grand-strategy-game",
    .version = "0.1.0",
    .dependencies = .{
        // Math & SIMD
        .zmath = .{
            .url = "https://github.com/michal-z/zig-gamedev/tree/main/libs/zmath",
        },
        .zlm = .{
            .url = "https://github.com/ziglibs/zlm", // Linear math
        },
        
        // Algorithms
        .zalgebra = .{
            .url = "https://github.com/kooparse/zalgebra",
        },
        
        // Data Structures
        .ziglang_data_structures = .{
            .url = "https://github.com/AssortedFantasy/ziglang-data-structures",
        },
        
        // Memory Allocators
        .zalloc = .{
            .url = "https://github.com/zigtools/zalloc",
        },
        
        // Noise Generation
        .znoise = .{
            .url = "https://github.com/ziglibs/znoise",
        },
        
        // Compression
        .zstd = .{
            .url = "https://github.com/facebook/zstd",
        },
        
        // Testing
        .ztest = .{
            .url = "https://github.com/karlseguin/ztest",
        },
    },
}
```

## Lua Libraries (via LuaRocks or embedded)

```lua
-- Package Management
rocks = {
    -- Core Libraries
    "penlight",      -- Comprehensive utilities
    "middleclass",   -- OOP support
    "moses",         -- Functional programming
    "lume",          -- Game dev utilities
    
    -- Data Handling
    "lua-cjson",     -- Fast JSON
    "msgpack",       -- MessagePack serialization
    "lua-protobuf",  -- Protocol Buffers
    "lyaml",         -- YAML support
    
    -- Math & Algorithms
    "lua-matrix",    -- Matrix operations
    "random",        -- Better random numbers
    "lrandom",       -- Mersenne Twister
    "perlin",        -- Noise generation
    
    -- Collections
    "lua-rbtree",    -- Red-black trees
    "heap",          -- Priority queues
    "deque",         -- Double-ended queues
    
    -- State Machines
    "statemachine",  -- FSM library
    "behavior3",     -- Behavior trees
    
    -- Rules Engine
    "lrexlib-pcre2", -- Advanced regex
    "lpeg",          -- Pattern matching
    "moonscript",    -- Alternative syntax
    
    -- Performance
    "luajit-msgpack-pure",
    "lua-cmsgpack",  -- C MessagePack
    
    -- Testing
    "busted",        -- BDD testing
    "luacheck",      -- Linting
    "luacov",        -- Code coverage
    
    -- Debugging
    "inspect",       -- Pretty printing
    "serpent",       -- Serialization
    "mobdebug",      -- Remote debugger
}
```
**RULES**
- Modular Architecture
- Modular functions
- Sophistication is achieved by brevity, not by complexity. The goal is to be extensible and as short but robust as possible for all functions and files.


## Phase 1: Core Engine Foundation

### Tauri Application Setup
- [x] Initialize Tauri 2.0 project with Rust backend and TypeScript frontend
- [x] Configure **Vite** with **@vitejs/plugin-react-swc** for fast HMR
- [x] Set up **@tauri-apps/api** for type-safe IPC communication
- [x] Configure **@tauri-apps/plugin-fs** for file system access
- [x] Implement custom window chrome with **@tauri-apps/plugin-window**
- [x] Configure app permissions and CSP policies (make light in beginning to ensure no breaking)
- [ ] Create installer configurations for each platform (lightweight)
- [ ] Implement crash reporting with **@sentry/react** (local mode)

### Backend Core Systems (Rust)
- [ ] **ECS Architecture Setup** (Bevy ECS or Legion)
  - [ ] Evaluate and choose **bevy_ecs** (best performance) vs **legion** (simpler API)
  - [ ] Design components with **serde** derive macros for serialization
  - [ ] Implement system scheduling with **rayon** parallel execution
  - [ ] Create resources with **parking_lot::RwLock** for thread safety
  - [ ] Build queries with **hecs** query optimization
  - [ ] Implement archetypes with **slotmap** for entity storage
  - [ ] Add change detection with **notify** crate
  - [ ] Create hierarchy with **petgraph** for entity relationships
  - [ ] Build serialization with **bincode** for saves
  - [ ] Implement hot-reload with **hot-lib-reloader**

- [ ] **Deterministic Simulation Core**
  - [ ] Implement fixed timestep with **spin_sleep** for precision
  - [ ] Create deterministic RNG with **rand_chacha::ChaCha8Rng**
  - [ ] Build float determinism with **ordered-float** (USE ZIG for SIMD)
  - [ ] Implement command queue with **crossbeam::channel**
  - [ ] Create snapshots with **rkyv** (zero-copy serialization)
  - [ ] Build replay system with **speedy** serialization
  - [ ] Add verification with **seahash** for checksums
  - [ ] Implement tick sync with **tokio::time::interval**
  - [ ] Create time control with **instant** crate
  - [ ] Build interpolation with **nalgebra::interpolation**

### Hex Grid System (Rust + Zig optimization)
- [ ] **Core Hex Mathematics** (IMPLEMENT IN ZIG with zmath)
  - [ ] Axial coordinates with **zmath** SIMD vectors
  - [ ] Cube conversions with **zalgebra** matrix ops
  - [ ] Offset conversions with Zig comptime optimization
  - [ ] Hex-to-pixel with **zmath.mat4** transformations
  - [ ] Distance calculations with SIMD batch processing
  - [ ] Line drawing with **bresenham** algorithm in Zig
  - [ ] FOV with **bracket-pathfinding** integration
  - [ ] Range finding with Zig vectorized loops
  - [ ] Rotations with **zmath** quaternions
  - [ ] Ring iterators with Zig comptime generation

- [ ] **Tile Storage Architecture** (Rust)
  - [ ] Chunk storage with **ndarray** for 2D arrays
  - [ ] Tile components with **hecs** sparse storage
  - [ ] Spatial indexing with **rstar** R-tree
  - [ ] Hierarchical tiles with **petgraph** DAG
  - [ ] Adjacency graph with **indexmap**
  - [ ] Edge detection with **image** crate algorithms
  - [ ] Ownership layers with **bitvec** for flags
  - [ ] Improvement slots with **slotmap**
  - [ ] Modifiers with **modular-bitfield**
  - [ ] Multi-layer with **arrayvec** fixed arrays

- [ ] **Tile Properties System** (Rust + Lua)
  - [ ] Terrain types with **strum** enums
  - [ ] Elevation with **noise** crate generation
  - [ ] Climate data with **nalgebra** interpolation
  - [ ] Biomes with **ron** data files
  - [ ] Resources with **serde** + **toml** configs
  - [ ] Improvements with **mlua** scripted effects
  - [ ] Movement costs with **fixedbitset**
  - [ ] Defense bonuses with **ordered-float**
  - [ ] Fog of war with **bitvec** visibility
  - [ ] Culture with **dashmap** concurrent maps

### Procedural World Generation (Rust + Lua for rules)
- [ ] **Noise Generation Systems** (USE ZIG with znoise for SIMD)
  - [ ] Simplex noise with **simdnoise** Rust + Zig
  - [ ] Perlin noise with **noise** crate
  - [ ] Voronoi with **voronator** for diagrams
  - [ ] Worley noise with **bracket-noise**
  - [ ] FBM with **zmath** SIMD operations
  - [ ] Domain warping with Zig vectorization
  - [ ] Ridged noise with **fast_noise_lite**
  - [ ] Noise mixing with **glam** SIMD math
  - [ ] Caching with **cached** proc macro
  - [ ] GPU fallback with **wgpu** compute shaders

- [ ] **Tectonic Simulation** (Rust)
  - [ ] Plate generation with **delaunator** triangulation
  - [ ] Movement vectors with **nalgebra::Vector2**
  - [ ] Boundaries with **geo** computational geometry
  - [ ] Mountain ranges with **line_drawing** algorithms
  - [ ] Rift valleys with **spade** spatial data
  - [ ] Transform faults with **robust** predicates
  - [ ] Plate age with **chrono** time tracking
  - [ ] Volcanic activity with **rand_distr** distributions
  - [ ] Earthquake maps with **ndarray** 2D arrays
  - [ ] Island chains with **convex_hull** algorithms

- [ ] **Climate and Biome Generation** (Lua rules with mlua)
  - [ ] Temperature zones with **penlight** Lua utilities
  - [ ] Ocean currents with **lume** game utilities
  - [ ] Wind patterns with Lua **moses** functional
  - [ ] Rainfall shadows with **middleclass** OOP
  - [ ] Seasonal variation with Lua **cron** patterns
  - [ ] Humidity with **lua-matrix** calculations
  - [ ] Biome determination with Lua decision trees
  - [ ] Transitions with **lpeg** pattern matching
  - [ ] Microclimates with Lua **random** module
  - [ ] Climate change with **statemachine** FSM

- [ ] **Hydrological Systems** (Rust)
  - [ ] Watersheds with **watershed** crate
  - [ ] River sources with **priority-queue**
  - [ ] Pathfinding with **pathfinding** A*
  - [ ] Flow accumulation with **ndarray**
  - [ ] River properties with **petgraph** graphs
  - [ ] Lakes with **union-find** structures
  - [ ] Wetlands with **kdtree** spatial queries
  - [ ] Flooding with **floodfill** algorithms
  - [ ] Aquifers with **grid_2d** storage
  - [ ] Springs with **rand::distributions**

- [ ] **Resource Distribution** (Lua configuration with mlua)
  - [ ] Resource types in Lua **moonscript** DSL
  - [ ] Geological rules with **lrexlib-pcre2** regex
  - [ ] Ore veins with Lua **perlin** noise
  - [ ] Oil deposits with **lua-protobuf** configs
  - [ ] Rare resources with **lrandom** RNG
  - [ ] Renewables with Lua **inspect** debugging
  - [ ] Quality with **serpent** serialization
  - [ ] Discovery with **behavior3** trees
  - [ ] Exhaustion with Lua **deque** queues
  - [ ] Scarcity with **lua-rbtree** balanced trees

### Frontend Rendering Pipeline (TypeScript + WebGL/WebGPU)
- [ ] **WebGL2/WebGPU Initialization** (Three.js + R3F)
  - [ ] Set up **@react-three/fiber** for declarative 3D scenes
  - [ ] Configure **@react-three/drei** for camera controls and helpers
  - [ ] Initialize WebGPU with WebGL2 fallback detection
  - [ ] Create **@react-three/postprocessing** pipeline
  - [ ] Set up **three-stdlib** for additional geometries
  - [ ] Implement device capability detection
  - [ ] Configure **leva** for runtime tweaking (dev mode)
  - [ ] Create render state management with **zustand**
  - [ ] Build performance monitoring with **stats.js**
  - [ ] Set up **theatre.js** for cinematic sequences

- [ ] **Procedural Hex Mesh Generation** (GPU-based)
  - [ ] Write GLSL shaders with **glslify** for modular shader code
  - [ ] Implement instanced rendering with **three-mesh-bvh**
  - [ ] Create per-instance data streaming
  - [ ] Build LOD system with **three-lod**
  - [ ] Use **troika-three-text** for performant text rendering
  - [ ] Implement hex edge detection shaders
  - [ ] Create border rendering with **three-line2**
  - [ ] Build terrain blending with custom shaders
  - [ ] Implement height-based coloring
  - [ ] Create water animation with **three-shader-toys**

- [ ] **Multi-pass Rendering System**
  - [ ] Set up **postprocessing** effect composer
  - [ ] Create shadow mapping with **three-csm** (cascaded shadows)
  - [ ] Build SSAO with **n8ao** (N8 ambient occlusion)
  - [ ] Implement TAA with **@react-three/postprocessing**
  - [ ] Create bloom effect with selective rendering
  - [ ] Build depth of field with bokeh shaders
  - [ ] Implement FXAA/SMAA anti-aliasing
  - [ ] Create HDR rendering pipeline
  - [ ] Build tone mapping with ACES
  - [ ] Implement fog with volumetric shaders

- [ ] **Frustum Culling System** (USE ZIG FOR SIMD)
  - [ ] Extract frustum planes from view-projection
  - [ ] Implement AABB frustum tests
  - [ ] Create sphere frustum tests
  - [ ] Build hierarchical culling
  - [ ] Implement chunk-based culling
  - [ ] Create occlusion culling system
  - [ ] Build LOD selection logic
  - [ ] Implement predictive culling
  - [ ] Create culling statistics
  - [ ] Build debug visualization

### Camera and Control Systems
- [ ] **Camera Implementation** (Three.js + Drei)
  - [ ] Create orbital camera with **@react-three/drei OrbitControls**
  - [ ] Implement free camera with **@react-three/drei FlyControls**
  - [ ] Build cinematic camera with **camera-controls** library
  - [ ] Create smooth interpolation with **framer-motion** 3D
  - [ ] Implement camera constraints with custom hooks
  - [ ] Build zoom management with **@use-gesture/react**
  - [ ] Create camera shake with **three-camera-shake**
  - [ ] Implement focus tracking with **@react-three/drei PivotControls**
  - [ ] Build camera bookmarks with **zustand** persistence
  - [ ] Create minimap camera with separate render target

- [ ] **Input Handling** (React + Gesture Libraries)
  - [ ] Implement mouse input with **@use-gesture/react**
  - [ ] Create keyboard shortcuts with **react-hotkeys-hook**
  - [ ] Build edge scrolling with custom React hooks
  - [ ] Implement pan gestures with **@use-gesture/react**
  - [ ] Create zoom handling with **hammerjs** integration
  - [ ] Build selection box with **react-selecto**
  - [ ] Implement context menus with **@radix-ui/react-context-menu**
  - [ ] Create tooltips with **@radix-ui/react-tooltip**
  - [ ] Build gesture recognition with **interactjs**
  - [ ] Implement gamepad with **gamecontroller.js**

### Save/Load System
- [ ] **Save File Management** (Tauri FS + Compression)
  - [ ] Design save format with **msgpackr** for binary serialization
  - [ ] Implement compression with **fflate** (pure JS, fast)
  - [ ] Create version management with **semver**
  - [ ] Build autosave with **node-cron** patterns
  - [ ] Implement quicksave with **@tauri-apps/plugin-fs**
  - [ ] Create save validation with **zod** schemas
  - [ ] Build thumbnails with **html-to-image**
  - [ ] Store metadata with **dexie** (IndexedDB wrapper)
  - [ ] Implement save encryption with **crypto-js**
  - [ ] Build save repair with custom recovery logic

### IPC Communication Layer
- [ ] **Tauri Command System** (Type-safe IPC)
  - [ ] Define commands with **@tauri-apps/api/tauri**
  - [ ] Create type definitions with **zod** schemas
  - [ ] Implement state queries with **@tanstack/react-query**
  - [ ] Build action commands with **zustand** actions
  - [ ] Create batch operations with **p-queue**
  - [ ] Implement validation with **zod** runtime checks
  - [ ] Build command history with **immer** patches
  - [ ] Create undo/redo with **redux-undo** pattern
  - [ ] Monitor performance with custom metrics
  - [ ] Build debug commands with **console-feed**

- [ ] **Event System** (Reactive Updates)
  - [ ] Design events with **eventemitter3**
  - [ ] Implement state sync with **valtio** proxies
  - [ ] Create notifications with **react-hot-toast**
  - [ ] Build progress updates with **nprogress**
  - [ ] Handle errors with **react-error-boundary**
  - [ ] Create performance monitoring with **web-vitals**
  - [ ] Build debug events with Chrome DevTools protocol
  - [ ] Implement filtering with **sift.js**
  - [ ] Create event recording with **rrweb**
  - [ ] Build replay system with recorded events

## Phase 2: Simulation Core

### Economic Simulation (Rust + Lua for rules)
- [ ] **Resource System** (Rust with serde)
  - [ ] Resource types with **strum** enum derive
  - [ ] Storage with **typed-arena** allocators
  - [ ] Production rates with **fixed** decimal math
  - [ ] Consumption with **priority-queue** scheduling
  - [ ] Conversion chains with **petgraph** DAG
  - [ ] Quality levels with **ordered-float**
  - [ ] Spoilage with **chrono::Duration** timers
  - [ ] Transportation with **pathfinding** crate
  - [ ] Stockpiling with **circular-buffer**
  - [ ] Reserves with **parking_lot::Mutex**

- [ ] **Production Chains** (Lua scripted with mlua)
  - [ ] Building types in Lua with **middleclass** OOP
  - [ ] Input/output ratios with **lua-matrix** math
  - [ ] Efficiency with **penlight** utilities
  - [ ] Worker requirements with Lua tables
  - [ ] Tech prerequisites with **behavior3** trees
  - [ ] Production queue with **deque** structure
  - [ ] Automation with **statemachine** FSM
  - [ ] Quality with **random** distributions
  - [ ] Seasonal with **cron** expressions
  - [ ] Statistics with **moses** functional ops

- [ ] **Market Simulation** (Zig for performance with zmath)
  - [ ] Supply/demand curves with Zig SIMD math
  - [ ] Price discovery with **zmath** vectors
  - [ ] Volatility with Zig random generators
  - [ ] Futures with Zig temporal calculations
  - [ ] Market manipulation detection in Zig
  - [ ] Black market with Zig probability
  - [ ] Embargoes with Zig bitflags
  - [ ] Currency with Zig fixed-point math
  - [ ] Inflation with Zig exponential smoothing
  - [ ] Indicators with Zig statistical functions

- [ ] **Trade System** (Rust)
  - [ ] Route finding with **hierarchical_pathfinding**
  - [ ] Caravan units with **slotmap** handles
  - [ ] Trade posts with **indexmap** lookups
  - [ ] Agreements with **ron** serialization
  - [ ] Tariffs with **rust_decimal** precision
  - [ ] Smuggling with **rand_distr** probability
  - [ ] Protection with **bitvec** flags
  - [ ] Piracy with **turborand** fast RNG
  - [ ] Statistics with **statrs** library
  - [ ] Visualization prep with **serde_json**

### Population System
- [ ] **Demographics** (State Management + Visualization)
  - [ ] Implement population units with **immer** for immutability
  - [ ] Create age pyramids with **d3-shape** and **visx**
  - [ ] Build gender distribution with **recharts** pie charts
  - [ ] Implement culture groups with **force-graph** clustering
  - [ ] Create religion maps with **deck.gl** layers
  - [ ] Build education UI with **react-circular-progressbar**
  - [ ] Implement social classes with **nivo** treemap
  - [ ] Create occupation charts with **apexcharts**
  - [ ] Build wealth distribution with **d3-scale**
  - [ ] Implement health metrics with **react-gauge-chart**

- [ ] **Population Dynamics** (Real-time Updates)
  - [ ] Create birth animations with **framer-motion**
  - [ ] Implement death transitions with **react-transition-group**
  - [ ] Build migration flows with **react-flow-renderer**
  - [ ] Create urbanization viz with **mapbox-gl**
  - [ ] Implement disease spread with **deck.gl** heatmap
  - [ ] Build famine effects with color transitions
  - [ ] Create casualty counters with **react-countup**
  - [ ] Implement refugee paths with **react-spring**
  - [ ] Build population graphs with **uplot**
  - [ ] Create demographic transitions with **d3-interpolate**

- [ ] **Needs and Happiness** (LUA CONFIGURABLE)
  - [ ] Define basic needs hierarchy
  - [ ] Implement luxury needs
  - [ ] Create need satisfaction
  - [ ] Build happiness calculation
  - [ ] Implement unrest system
  - [ ] Create loyalty mechanics
  - [ ] Build protest system
  - [ ] Implement revolution triggers
  - [ ] Create happiness modifiers
  - [ ] Build quality of life index

### City Management
- [ ] **District System**
  - [ ] Create district types
  - [ ] Implement district placement
  - [ ] Build adjacency bonuses
  - [ ] Create district specialization
  - [ ] Implement district levels
  - [ ] Build district population
  - [ ] Create district maintenance
  - [ ] Implement district damage
  - [ ] Build district repair
  - [ ] Create district conversion

- [ ] **Building System**
  - [ ] Define building types and tiers
  - [ ] Implement building placement rules
  - [ ] Create building prerequisites
  - [ ] Build construction queue
  - [ ] Implement building maintenance
  - [ ] Create building upgrades
  - [ ] Build building bonuses
  - [ ] Implement building decay
  - [ ] Create unique buildings
  - [ ] Build wonder construction

- [ ] **Infrastructure Networks**
  - [ ] Implement road network
  - [ ] Create aqueduct system
  - [ ] Build power grid (late game)
  - [ ] Implement sewer system
  - [ ] Create transportation hubs
  - [ ] Build communication network
  - [ ] Implement supply distribution
  - [ ] Create emergency services
  - [ ] Build fortification network
  - [ ] Implement underground tunnels

### Environmental Systems
- [ ] **Pollution and Climate**
  - [ ] Create pollution sources
  - [ ] Implement pollution spread
  - [ ] Build air quality system
  - [ ] Create water pollution
  - [ ] Implement soil contamination
  - [ ] Build cleanup mechanics
  - [ ] Create climate effects
  - [ ] Implement global warming
  - [ ] Build renewable energy
  - [ ] Create environmental policies

- [ ] **Natural Disasters** (LUA EVENT SCRIPTS)
  - [ ] Implement earthquakes
  - [ ] Create volcanic eruptions
  - [ ] Build tsunami system
  - [ ] Implement hurricanes
  - [ ] Create tornadoes
  - [ ] Build flooding system
  - [ ] Implement droughts
  - [ ] Create wildfires
  - [ ] Build plague outbreaks
  - [ ] Implement meteor strikes

## Phase 3: Governance and Politics

### Government Systems (Rust + Lua configuration)
- [ ] **Government Types** (Rust with mlua)
  - [ ] Tribal government with **strum** enums
  - [ ] Monarchy with **serde** serialization
  - [ ] Oligarchy with **ron** config files
  - [ ] Democracy with **petgraph** voting graphs
  - [ ] Theocracy with **bitflags** permissions
  - [ ] Military dictatorship with **chain-of-command**
  - [ ] Communist state with **collective** crate
  - [ ] Anarchist communes with **consensus** algorithms
  - [ ] Corporate government with **shares** tracking
  - [ ] AI technocracy with **decision-tree** crate

- [ ] **Power Structure** (Rust)
  - [ ] Executive branch with **authority** crate
  - [ ] Legislative with **voting** crate
  - [ ] Judicial with **precedent** tracking
  - [ ] Bureaucracy with **hierarchy** trees
  - [ ] Federal/state with **multi-level** governance
  - [ ] Power balance with **game-theory** crate
  - [ ] Coup mechanics with **probability** crate
  - [ ] Succession with **inheritance** rules
  - [ ] Legitimacy with **reputation** system
  - [ ] Stability with **equilibrium** calculations

- [ ] **Power Structure**
  - [ ] Create executive branch
  - [ ] Implement legislative system
  - [ ] Build judicial branch
  - [ ] Create bureaucracy mechanics
  - [ ] Implement federal/state/local
  - [ ] Build power balance system
  - [ ] Create coup mechanics
  - [ ] Implement succession rules
  - [ ] Build legitimacy system
  - [ ] Create government stability

- [ ] **Policy System** (LUA SCRIPTED)
  - [ ] Define policy categories
  - [ ] Implement policy cards/slots
  - [ ] Create policy prerequisites
  - [ ] Build policy effects
  - [ ] Implement policy combos
  - [ ] Create policy penalties
  - [ ] Build policy duration
  - [ ] Implement policy popularity
  - [ ] Create policy momentum
  - [ ] Build policy reversals

### Internal Politics
- [ ] **Political Parties**
  - [ ] Create party generation
  - [ ] Implement party ideologies
  - [ ] Build party popularity
  - [ ] Create party coalitions
  - [ ] Implement party splits
  - [ ] Build party funding
  - [ ] Create party corruption
  - [ ] Implement party leaders
  - [ ] Build party platforms
  - [ ] Create party events

- [ ] **Elections and Voting**
  - [ ] Implement voting systems
  - [ ] Create campaign mechanics
  - [ ] Build polling system
  - [ ] Implement voter turnout
  - [ ] Create electoral fraud
  - [ ] Build gerrymandering
  - [ ] Implement referendums
  - [ ] Create recall elections
  - [ ] Build term limits
  - [ ] Implement lame duck periods

- [ ] **Interest Groups** (LUA DEFINED)
  - [ ] Create military faction
  - [ ] Implement merchant guilds
  - [ ] Build religious orders
  - [ ] Create labor unions
  - [ ] Implement noble houses
  - [ ] Build scholar societies
  - [ ] Create criminal syndicates
  - [ ] Implement ethnic groups
  - [ ] Build environmental groups
  - [ ] Create secret societies

### Legal System
- [ ] **Laws and Constitution**
  - [ ] Create law categories
  - [ ] Implement law passage
  - [ ] Build constitutional system
  - [ ] Create amendments process
  - [ ] Implement emergency powers
  - [ ] Build martial law
  - [ ] Create law enforcement
  - [ ] Implement court system
  - [ ] Build prison system
  - [ ] Create rehabilitation programs

- [ ] **Crime and Justice**
  - [ ] Implement crime types
  - [ ] Create crime rates
  - [ ] Build investigation system
  - [ ] Implement trial mechanics
  - [ ] Create punishment system
  - [ ] Build corruption mechanics
  - [ ] Implement vigilante justice
  - [ ] Create organized crime
  - [ ] Build witness protection
  - [ ] Implement rehabilitation

### Diplomacy (AI-only)
- [ ] **Diplomatic Relations**
  - [ ] Create opinion system (-100 to +100)
  - [ ] Implement trust mechanics
  - [ ] Build reputation system
  - [ ] Create diplomatic range
  - [ ] Implement first contact
  - [ ] Build embassy system
  - [ ] Create diplomatic immunity
  - [ ] Implement persona non grata
  - [ ] Build diplomatic incidents
  - [ ] Create casus belli system

- [ ] **Diplomatic Actions**
  - [ ] Implement trade deals
  - [ ] Create alliance system
  - [ ] Build defensive pacts
  - [ ] Implement non-aggression
  - [ ] Create vassalage system
  - [ ] Build tributary states
  - [ ] Implement guarantees
  - [ ] Create ultimatums
  - [ ] Build peace treaties
  - [ ] Implement reparations

- [ ] **AI Personality System** (LUA CONFIGURABLE)
  - [ ] Create personality archetypes
  - [ ] Implement aggressiveness scale
  - [ ] Build trustworthiness
  - [ ] Create greed factor
  - [ ] Implement honor code
  - [ ] Build pragmatism level
  - [ ] Create xenophobia scale
  - [ ] Implement militarism
  - [ ] Build trade focus
  - [ ] Create cultural focus

## Phase 4: Military Systems

### Unit Systems (Rust core, Zig for combat math)
- [ ] **Unit Types and Roles**
  - [ ] Create infantry units
  - [ ] Implement cavalry/mobile
  - [ ] Build ranged units
  - [ ] Create siege units
  - [ ] Implement support units
  - [ ] Build naval units
  - [ ] Create air units (late game)
  - [ ] Implement space units (late game)
  - [ ] Build special forces
  - [ ] Create militia/irregulars

- [ ] **Unit Statistics** (ZIG FOR COMBAT CALCULATIONS)
  - [ ] Implement attack values
  - [ ] Create defense values
  - [ ] Build movement points
  - [ ] Implement hit points
  - [ ] Create morale system
  - [ ] Build experience levels
  - [ ] Implement supply needs
  - [ ] Create maintenance costs
  - [ ] Build upgrade paths
  - [ ] Implement veterancy bonuses

- [ ] **Unit Designer**
  - [ ] Create modular components
  - [ ] Implement chassis system
  - [ ] Build weapon slots
  - [ ] Create armor options
  - [ ] Implement engine types
  - [ ] Build special equipment
  - [ ] Create cost calculation
  - [ ] Implement design saving
  - [ ] Build design sharing
  - [ ] Create design evolution

### Combat System (Zig for performance)
- [ ] **Battle Resolution** (Zig with zmath)
  - [ ] Initiative order with Zig sorting algorithms
  - [ ] Combat phases with Zig state machines
  - [ ] Damage calculation with **zmath** SIMD
  - [ ] Armor penetration with Zig fixed-point math
  - [ ] Critical hits with Zig PRNG
  - [ ] Morale checks with Zig thresholds
  - [ ] Retreats with Zig pathfinding
  - [ ] Pursuit with Zig movement calculation
  - [ ] Combat logging with Zig ring buffer
  - [ ] Battle replay with Zig serialization

- [ ] **Tactical Elements** (Zig optimization)
  - [ ] Terrain bonuses with Zig lookup tables
  - [ ] Elevation advantage with **zalgebra** vectors
  - [ ] Flanking with Zig geometry calculations
  - [ ] Encirclement with Zig convex hull
  - [ ] Ambushes with Zig visibility checks
  - [ ] Fortification with Zig defense modifiers
  - [ ] Weather impacts with Zig probability
  - [ ] Day/night with Zig time calculations
  - [ ] Supply cuts with Zig graph algorithms
  - [ ] Combined arms with Zig unit synergy

- [ ] **Siege Warfare**
  - [ ] Implement siege progress
  - [ ] Create wall breaching
  - [ ] Build siege equipment
  - [ ] Implement blockades
  - [ ] Create starvation mechanics
  - [ ] Build assault options
  - [ ] Implement tunneling
  - [ ] Create bombardment
  - [ ] Build surrender terms
  - [ ] Implement sacking cities

### Military Organization
- [ ] **Army Management**
  - [ ] Create unit stacking
  - [ ] Implement army composition
  - [ ] Build command structure
  - [ ] Create supply trains
  - [ ] Implement reinforcements
  - [ ] Build army traditions
  - [ ] Create military doctrine
  - [ ] Implement mobilization
  - [ ] Build reserve system
  - [ ] Create mercenary companies

- [ ] **Military Logistics**
  - [ ] Implement supply production
  - [ ] Create supply distribution
  - [ ] Build supply depots
  - [ ] Implement foraging
  - [ ] Create equipment wear
  - [ ] Build repair systems
  - [ ] Implement ammunition
  - [ ] Create medical corps
  - [ ] Build field hospitals
  - [ ] Implement logistics efficiency

### Naval and Air Systems
- [ ] **Naval Combat**
  - [ ] Create ship types
  - [ ] Implement naval movement
  - [ ] Build naval combat
  - [ ] Create boarding actions
  - [ ] Implement ramming
  - [ ] Build naval bombardment
  - [ ] Create blockade system
  - [ ] Implement convoy raiding
  - [ ] Build naval invasions
  - [ ] Create naval supply

- [ ] **Air Combat** (Late Game)
  - [ ] Implement air units
  - [ ] Create air superiority
  - [ ] Build bombing runs
  - [ ] Implement reconnaissance
  - [ ] Create air transport
  - [ ] Build air supply
  - [ ] Implement interception
  - [ ] Create escort missions
  - [ ] Build anti-air systems
  - [ ] Implement missile systems

## Phase 5: Advanced Features

### Technology System (Rust + Lua)
- [ ] **Research Mechanics**
  - [ ] Create tech tree structure
  - [ ] Implement research points
  - [ ] Build research allocation
  - [ ] Create parallel research
  - [ ] Implement tech prerequisites
  - [ ] Build tech trading
  - [ ] Create tech stealing
  - [ ] Implement reverse engineering
  - [ ] Build eureka moments
  - [ ] Create tech obsolescence

- [ ] **Technology Eras** (LUA CONFIGURED)
  - [ ] Implement ancient era
  - [ ] Create classical era
  - [ ] Build medieval era
  - [ ] Implement renaissance era
  - [ ] Create industrial era
  - [ ] Build modern era
  - [ ] Implement information era
  - [ ] Create future era
  - [ ] Build era transitions
  - [ ] Implement era bonuses

- [ ] **Technology UI** (Graph Visualization)
  - [ ] Create tech tree with **cytoscape.js**
  - [ ] Implement progress bars with **react-circular-progressbar**
  - [ ] Build research queue with **react-beautiful-dnd**
  - [ ] Create era transitions with **framer-motion**
  - [ ] Implement tooltips with **@floating-ui/react**
  - [ ] Build search with **fuse.js**
  - [ ] Create filtering with **react-select**
  - [ ] Implement zoom controls with **react-zoom-pan-pinch**
  - [ ] Build minimap with **react-minimap**
  - [ ] Create export with **html2canvas**

### Culture System
- [ ] **Cultural Identity**
  - [ ] Create cultural traits
  - [ ] Implement cultural values
  - [ ] Build cultural traditions
  - [ ] Create cultural evolution
  - [ ] Implement cultural drift
  - [ ] Build cultural merger
  - [ ] Create cultural revival
  - [ ] Implement cultural golden age
  - [ ] Build cultural dark age
  - [ ] Create cultural achievements

- [ ] **Cultural Spread** (ZIG FOR CALCULATION)
  - [ ] Implement influence calculation
  - [ ] Create distance decay
  - [ ] Build trade route bonus
  - [ ] Implement tourism mechanics
  - [ ] Create cultural pressure
  - [ ] Build cultural conversion
  - [ ] Implement cultural resistance
  - [ ] Create cultural borders
  - [ ] Build cultural dominance
  - [ ] Implement soft power

- [ ] **Great Works and Art**
  - [ ] Create great work types
  - [ ] Implement great artists
  - [ ] Build museums/galleries
  - [ ] Create theming bonuses
  - [ ] Implement art trading
  - [ ] Build art theft
  - [ ] Create forgeries
  - [ ] Implement archaeology
  - [ ] Build artifact system
  - [ ] Create cultural heritage

### Religion System (Lua for beliefs)
- [ ] **Religious Mechanics**
  - [ ] Create pantheon system
  - [ ] Implement prophet units
  - [ ] Build religious founding
  - [ ] Create belief selection
  - [ ] Implement religious spread
  - [ ] Build conversion mechanics
  - [ ] Create religious combat
  - [ ] Implement inquisition
  - [ ] Build reformation
  - [ ] Create schisms

- [ ] **Religious Infrastructure**
  - [ ] Implement shrines
  - [ ] Create temples
  - [ ] Build cathedrals
  - [ ] Implement monasteries
  - [ ] Create holy sites
  - [ ] Build pilgrimage routes
  - [ ] Implement religious orders
  - [ ] Create missionary units
  - [ ] Build apostle units
  - [ ] Implement religious relics

### Espionage System
- [ ] **Spy Operations**
  - [ ] Create spy units
  - [ ] Implement spy recruitment
  - [ ] Build spy training
  - [ ] Create spy placement
  - [ ] Implement surveillance
  - [ ] Build sabotage missions
  - [ ] Create theft operations
  - [ ] Implement assassinations
  - [ ] Build coup support
  - [ ] Create false flag operations

- [ ] **Counter-Intelligence**
  - [ ] Implement security levels
  - [ ] Create counter-spy units
  - [ ] Build detection system
  - [ ] Implement interrogation
  - [ ] Create double agents
  - [ ] Build spy networks
  - [ ] Implement code breaking
  - [ ] Create disinformation
  - [ ] Build honeypot operations
  - [ ] Implement diplomatic immunity

## Phase 6: AI Systems

### Strategic AI (Rust + Lua personalities)
- [ ] **AI Architecture** (Rust)
  - [ ] GOAP with **goap** crate implementation
  - [ ] Behavior trees with **behavior-tree** crate
  - [ ] Utility AI with **utility-ai** crate
  - [ ] HTN planning with **htn** crate
  - [ ] Influence maps with **ndarray** 2D arrays
  - [ ] Threat assessment with **ordered-float** scoring
  - [ ] Opportunity eval with **priority-queue**
  - [ ] Memory with **circular-buffer** history
  - [ ] Learning with **smartcore** ML
  - [ ] Personalities with **ron** config files

- [ ] **AI Decision Making** (Lua configurable with mlua)
  - [ ] Economic planning with **penlight** utilities
  - [ ] Military strategy with **behavior3** trees
  - [ ] Diplomatic behavior with **statemachine** FSM
  - [ ] Expansion with Lua **heap** priority queue
  - [ ] Research priorities with **moses** functional
  - [ ] Cultural focus with **middleclass** OOP
  - [ ] Religious strategy with Lua patterns
  - [ ] Espionage with **lrandom** probabilities
  - [ ] Crisis management with **lpeg** matching
  - [ ] Long-term planning with Lua coroutines

- [ ] **AI Difficulty Levels** (Lua tuning)
  - [ ] Difficulty scaling with Lua multipliers
  - [ ] AI bonuses with **lyaml** configs
  - [ ] Handicaps with Lua conditional logic
  - [ ] Cheating options with **inspect** debugging
  - [ ] Smart difficulty with Lua adaptation
  - [ ] Adaptive AI with **serpent** state saving
  - [ ] Personalities in **moonscript** DSL
  - [ ] Historical AI with Lua event scripts
  - [ ] Random AI with **random** module
  - [ ] Custom AI with Lua hot-reload

### Tactical AI (Zig for performance)
- [ ] **Combat AI** (Zig with zmath)
  - [ ] Unit positioning with **zmath** vectors
  - [ ] Formations with Zig spatial algorithms
  - [ ] Target selection with Zig priority scoring
  - [ ] Retreat logic with Zig state machines
  - [ ] Flanking with **zalgebra** geometry
  - [ ] Siege tactics with Zig planning
  - [ ] Combined arms with Zig coordination
  - [ ] Terrain usage with Zig analysis
  - [ ] Defense with Zig positioning algorithms
  - [ ] Offense with Zig assault planning

- [ ] **Movement AI** (Zig optimized)
  - [ ] Pathfinding with Zig A* + JPS
  - [ ] Flow fields with Zig SIMD operations
  - [ ] Group movement with Zig flocking
  - [ ] Formation keeping with Zig constraints
  - [ ] Collision avoidance with Zig RVO
  - [ ] Strategic positioning with Zig heuristics
  - [ ] Patrol with Zig waypoint systems
  - [ ] Exploration with Zig frontier detection
  - [ ] Supply protection with Zig coverage
  - [ ] Reinforcement with Zig shortest paths

### Economic AI (Rust)
- [ ] **Resource Management** (Rust)
  - [ ] Production planning with **linear_programming**
  - [ ] Resource allocation with **good_lp** solver
  - [ ] Trade evaluation with **rust_decimal** math
  - [ ] Market manipulation with **statrs** statistics
  - [ ] Stockpile management with **moving_average**
  - [ ] Emergency reserves with **threshold** crate
  - [ ] Growth vs military with **multi-objective**
  - [ ] Infrastructure with **graph-algorithms**
  - [ ] Specialization with **kmeans** clustering
  - [ ] Optimization with **argmin** optimizer

- [ ] **City Planning AI** (Rust + Lua)
  - [ ] District placement with **placement** algorithms
  - [ ] Building priorities with **priority-queue**
  - [ ] Adjacency optimization with **simulated_annealing**
  - [ ] Growth management with **pid** controller
  - [ ] Happiness balance with **fuzzy-logic**
  - [ ] Defense planning with **voronoi** regions
  - [ ] Wonder timing with **scheduling** crate
  - [ ] Improvement selection with Lua rules
  - [ ] Road networks with **minimum_spanning_tree**
  - [ ] Trade routes with **network-flow** optimization

## Phase 7: User Interface

### Main UI Framework (React + TypeScript)
- [ ] **Window Management** (React + Layout Libraries)
  - [ ] Create dockable panels with **react-mosaic-component**
  - [ ] Implement resizable panes with **react-resizable-panels**  
  - [ ] Build window state with **zustand** persistence
  - [ ] Create snapping with **react-rnd** (resize and drag)
  - [ ] Implement tabs with **@radix-ui/react-tabs**
  - [ ] Build workspace saving with **dexie** (IndexedDB)
  - [ ] Create multi-monitor support via Tauri APIs
  - [ ] Implement UI scaling with **react-use-measure**
  - [ ] Build theme system with **next-themes**
  - [ ] Create accessibility with **react-aria-components**

- [ ] **Menu Systems** (Radix UI + Routing)
  - [ ] Implement main menu with **@radix-ui/react-navigation-menu**
  - [ ] Create game menu with **@radix-ui/react-dropdown-menu**
  - [ ] Build options with **react-hook-form** + **zod**
  - [ ] Implement save/load UI with **@tanstack/react-table**
  - [ ] Create scenario browser with **react-window** virtualization
  - [ ] Build mod manager with **react-beautiful-dnd**
  - [ ] Implement credits with **framer-motion** animations
  - [ ] Create achievements with **react-flip-toolkit**
  - [ ] Build statistics with **recharts** dashboards
  - [ ] Implement encyclopedia with **fuse.js** search

### HUD and Overlays
- [ ] **Resource Display** (Data Viz Libraries)
  - [ ] Create resource bars with **react-circular-progressbar**
  - [ ] Implement tooltips with **floating-ui**
  - [ ] Build alerts with **react-hot-toast**
  - [ ] Create graphs with **visx** mini charts
  - [ ] Implement flow viz with **react-flow**
  - [ ] Build predictions with **recharts** projections
  - [ ] Create trade balance with **d3** scales
  - [ ] Implement income UI with **react-countup**
  - [ ] Build stockpile with **framer-motion** gauges
  - [ ] Create warnings with **sonner** notifications

- [ ] **Information Overlays** (Three.js + Canvas)
  - [ ] Implement map overlays with **@deck.gl/react**
  - [ ] Create heat maps with **d3-scale-chromatic**
  - [ ] Build influence gradients with **three.js** shaders
  - [ ] Implement trade routes with **react-force-graph-3d**
  - [ ] Create fog of war with custom WebGL shaders
  - [ ] Build territory borders with **turf.js** geometry
  - [ ] Implement climate viz with **mapbox-gl**
  - [ ] Create military overlays with **pixi.js**
  - [ ] Build supply networks with **vis-network**
  - [ ] Implement resource dots with **three-instanced-mesh**

- [ ] **Notification System** (Toast + Queue Libraries)
  - [ ] Create notification queue with **react-hot-toast**
  - [ ] Implement priority sorting with **p-queue**
  - [ ] Build notification history with **dexie**
  - [ ] Create filtering with **match-sorter**
  - [ ] Implement grouping with **lodash-es** utilities
  - [ ] Build sound alerts with **howler.js**
  - [ ] Create visual alerts with **react-spring**
  - [ ] Implement badges with **react-intersection-observer**
  - [ ] Build settings UI with **react-hook-form**
  - [ ] Create smart notifications with AI filtering

### Detail Panels
- [ ] **City Management UI** (Complex Forms + Tables)
  - [ ] Create city overview with **@tanstack/react-table**
  - [ ] Implement production queue with **react-beautiful-dnd**
  - [ ] Build citizen management with **react-select**
  - [ ] Create building grid with **react-grid-layout**
  - [ ] Implement district view with **react-hexgrid**
  - [ ] Build growth charts with **recharts**
  - [ ] Create happiness UI with **react-gauge-chart**
  - [ ] Implement trade routes with **react-flow**
  - [ ] Build defense status with **react-circular-progressbar**
  - [ ] Create governor UI with **@dnd-kit/sortable**

- [ ] **Unit Control UI** (Interactive Components)
  - [ ] Implement unit cards with **framer-motion** 3D
  - [ ] Create action menus with **@radix-ui/react-context-menu**
  - [ ] Build promotion tree with **react-d3-tree**
  - [ ] Implement unit list with **@tanstack/react-virtual**
  - [ ] Create army manager with **react-sortablejs**
  - [ ] Build formation editor with **konva** canvas
  - [ ] Implement supply viz with **react-sparklines**
  - [ ] Create unit designer with **react-color** + **react-dnd**
  - [ ] Build upgrade panel with **react-step-progress-bar**
  - [ ] Implement unit history with **react-chrono**

- [ ] **Diplomacy Screen** (Network Graphs + Forms)
  - [ ] Create leader portraits with **react-avatar**
  - [ ] Implement relationship web with **vis-network**
  - [ ] Build deal maker with **react-hook-form**
  - [ ] Create trade UI with **react-select** multi
  - [ ] Implement alliance viz with **react-force-graph-2d**
  - [ ] Build war/peace UI with **framer-motion** transitions
  - [ ] Create espionage panel with **react-circular-menu**
  - [ ] Implement world congress with **react-org-chart**
  - [ ] Build victory progress with **react-step-progress**
  - [ ] Create history timeline with **react-calendar-timeline**

### Charts and Graphs
- [ ] **Statistics Displays** (D3 + Visualization Libraries)
  - [ ] Create line graphs with **visx** curves
  - [ ] Implement bar charts with **recharts**
  - [ ] Build pie charts with **nivo** pie
  - [ ] Create heat maps with **react-heatmap-grid**
  - [ ] Implement scatter plots with **plotly.js**
  - [ ] Build sankey diagrams with **@nivo/sankey**
  - [ ] Create treemaps with **d3-hierarchy**
  - [ ] Implement histograms with **visx** bars
  - [ ] Build radar charts with **react-chartjs-2**
  - [ ] Create comparison tools with **react-compare-slider**

- [ ] **Historical Data** (Time Series + Animation)
  - [ ] Implement score tracking with **uplot**
  - [ ] Create demographic pyramids with **d3-shape**
  - [ ] Build economic history with **lightweight-charts**
  - [ ] Implement military timeline with **vis-timeline**
  - [ ] Create cultural evolution with **rough-viz**
  - [ ] Build tech progress with **react-sweet-progress**
  - [ ] Implement territory animation with **react-map-gl**
  - [ ] Create relationship history with **cytoscape**
  - [ ] Build event timeline with **react-vertical-timeline**
  - [ ] Implement replay with **rrweb-player**

## Phase 8: Content and Polish

### Audio System (TypeScript + Web Audio)
- [ ] **Music System** (Tone.js + Howler)
  - [ ] Implement dynamic music with **tone.js** scheduling
  - [ ] Create era-based tracks with **howler.js** sprites
  - [ ] Build situation music with **tone.js** Transport
  - [ ] Implement ambient loops with **howler.js** fading
  - [ ] Create victory/defeat stingers with **tone.js** samplers
  - [ ] Build combat music with **tone.js** patterns
  - [ ] Implement crossfading with **howler.js** groups
  - [ ] Create cultural music with **tone.js** instruments
  - [ ] Build volume control with **standardized-audio-context**
  - [ ] Implement music preferences with **zustand** persistence

- [ ] **Sound Effects** (Web Audio API + Howler)
  - [ ] Create UI sounds with **howler.js** sprites
  - [ ] Implement unit sounds with **pizzicato.js** effects
  - [ ] Build combat sounds with **tone.js** synthesis
  - [ ] Create building sounds with **howler.js** spatial
  - [ ] Implement ambient with **web-audio-beat-detector**
  - [ ] Build notifications with **use-sound** React hook
  - [ ] Create voice clips with **wavesurfer.js**
  - [ ] Implement 3D audio with **resonance-audio**
  - [ ] Build sound priorities with **p-queue**
  - [ ] Create settings UI with **react-use-audio**

### Visual Polish
- [ ] **Particle Effects** (Three.js + Canvas)
  - [ ] Implement combat particles with **three-nebula**
  - [ ] Create building smoke with **three.js** GPUParticles
  - [ ] Build weather with **react-three-fiber** particles
  - [ ] Implement fire with **three-particle-fire**
  - [ ] Create explosions with **three-explosion**
  - [ ] Build magic effects with **proton.js**
  - [ ] Implement dust clouds with **particlesjs**
  - [ ] Create water splashes with **liquidfun.js**
  - [ ] Build UI particles with **tsparticles**
  - [ ] Implement celebrations with **canvas-confetti**

- [ ] **Shader Effects** (GLSL + Three.js)
  - [ ] Create water shaders with **three-shader-water**
  - [ ] Implement terrain with **three-terrain-shader**
  - [ ] Build fog with **three.js** fog shaders
  - [ ] Create outlines with **three-outline-effect**
  - [ ] Implement glow with **three-glow-mesh**
  - [ ] Build distortion with **three-distortion-material**
  - [ ] Create transitions with **gl-transitions**
  - [ ] Implement post-processing with **postprocessing**
  - [ ] Build weather shaders with **three-sky-shader**
  - [ ] Create special effects with **shadertoy** ports

- [ ] **Animation System** (Framer Motion + Three.js)
  - [ ] Implement unit animations with **@react-three/drei** helpers
  - [ ] Create building animations with **react-spring/three**
  - [ ] Build UI animations with **framer-motion**
  - [ ] Implement camera moves with **camera-controls**
  - [ ] Create combat animations with **animejs**
  - [ ] Build idle animations with **lottie-react**
  - [ ] Implement death animations with **react-transition-group**
  - [ ] Create celebrations with **react-rewards**
  - [ ] Build weather animations with **react-rain-animation**
  - [ ] Implement smooth transitions with **auto-animate**

### Procedural Content Generation (Lua scripts)
- [ ] **Name Generators**
  - [ ] Create culture-based names
  - [ ] Implement place names
  - [ ] Build character names
  - [ ] Create unit names
  - [ ] Implement city names
  - [ ] Build nation names
  - [ ] Create dynasty names
  - [ ] Implement title generation
  - [ ] Build epithet system
  - [ ] Create naming rules

- [ ] **Event Generation**
  - [ ] Implement random events
  - [ ] Create historical events
  - [ ] Build chain events
  - [ ] Implement crisis events
  - [ ] Create opportunity events
  - [ ] Build narrative events
  - [ ] Implement decision events
  - [ ] Create discovery events
  - [ ] Build disaster events
  - [ ] Implement golden age events

- [ ] **Quest Generation**
  - [ ] Create quest templates
  - [ ] Implement quest chains
  - [ ] Build victory quests
  - [ ] Create side quests
  - [ ] Implement hidden quests
  - [ ] Build timed quests
  - [ ] Create repeatable quests
  - [ ] Implement faction quests
  - [ ] Build achievement quests
  - [ ] Create tutorial quests

## Phase 9: Performance Optimization

### Backend Optimization (Rust + Zig)
- [ ] **Memory Management** (Rust)
  - [ ] Custom allocators with **bumpalo** arenas
  - [ ] Object pools with **object-pool** crate
  - [ ] Arena allocation with **typed-arena**
  - [ ] Reference counting with **arc-swap**
  - [ ] Garbage-free with **generational-arena**
  - [ ] Memory budgets with **memory-stats** tracking
  - [ ] Cache optimization with **crossbeam-epoch**
  - [ ] Memory profiling with **dhat** heap profiler
  - [ ] Leak detection with **leak-detect-allocator**
  - [ ] Memory reporting with **sys-info** crate

- [ ] **Parallel Processing** (USE ZIG for SIMD with zmath)
  - [ ] Thread pools with **rayon::ThreadPool**
  - [ ] Work stealing with **crossbeam-deque**
  - [ ] Parallel queries with **rayon::par_iter**
  - [ ] SIMD math with Zig **zmath** library
  - [ ] Batch processing with **par-stream**
  - [ ] Async tasks with **tokio::spawn**
  - [ ] Lock-free with **crossbeam-channel**
  - [ ] Atomics with **atomic** crate types
  - [ ] Parallel pathfinding with **rayon-hash**
  - [ ] Concurrent updates with **dashmap**

- [ ] **Data Optimization** (Rust + Zig)
  - [ ] Compression with **zstd** + **lz4_flex**
  - [ ] Delta encoding with **bitpack** crate
  - [ ] Sparse storage with **sparse-vec**
  - [ ] Bitpacking with **bitpacking** crate
  - [ ] Quantization with Zig fixed-point
  - [ ] Data streaming with **futures-stream**
  - [ ] Predictive loading with **prefetch** hints
  - [ ] Cache warming with **ahash** hasher
  - [ ] Data prefetching with **crossbeam-cache**
  - [ ] Hot/cold separation with **mini-moka** cache

### Frontend Optimization (TypeScript)
- [ ] **Rendering Optimization** (WebGL + React)
  - [ ] Implement batch rendering with **three-batch-manager**
  - [ ] Create instancing with **three-instanced-mesh**
  - [ ] Build LOD with **three-lod-system**
  - [ ] Implement culling with **three-cull-manager**
  - [ ] Create texture atlases with **texture-packer**
  - [ ] Build mesh optimization with **meshoptimizer**
  - [ ] Implement GPU skinning with **three.js** skinning
  - [ ] Create render queues with **three-render-queue**
  - [ ] Build state sorting with **webgl-state-cache**
  - [ ] Implement draw call batching with **three-merge-geometries**

- [ ] **Web Workers** (Comlink + WorkerPool)
  - [ ] Create worker pool with **workerpool**
  - [ ] Implement pathfinding worker with **comlink**
  - [ ] Build physics worker with **oimo.js** in worker
  - [ ] Create AI worker with **comlink** RPC
  - [ ] Implement generation worker with **threads.js**
  - [ ] Build compression worker with **fflate** in worker
  - [ ] Create audio worker with **audio-worklet**
  - [ ] Implement network worker with **comlink**
  - [ ] Build computation worker with **gpu.js**
  - [ ] Create background tasks with **queue-microtask**

- [ ] **Asset Optimization** (Build Tools + Loaders)
  - [ ] Implement lazy loading with **react-lazy-load**
  - [ ] Create bundles with **vite** chunking
  - [ ] Build texture compression with **basis_universal**
  - [ ] Implement mesh decimation with **simplify-js**
  - [ ] Create audio compression with **lamejs**
  - [ ] Build font subsetting with **fontmin**
  - [ ] Implement sprite sheets with **spritesmith**
  - [ ] Create mipmaps with **jimp**
  - [ ] Build asset streaming with **progressive-loader**
  - [ ] Implement caching with **workbox**

### Profiling and Debugging
- [ ] **Performance Monitoring** (Metrics + Analytics)
  - [ ] Create FPS counter with **stats.js**
  - [ ] Implement frame timing with **raf-perf**
  - [ ] Build performance graphs with **perf-monitor**
  - [ ] Create memory monitor with **memory-stats.js**
  - [ ] Implement CPU profiler with Chrome DevTools API
  - [ ] Build GPU profiler with **webgl-debug**
  - [ ] Create network monitor with **network-information-api**
  - [ ] Implement disk monitor with **@tauri-apps/api/fs**
  - [ ] Build battery monitor with **battery-api-wrapper**
  - [ ] Create thermal monitor with system APIs

- [ ] **Debug Tools** (DevTools + Inspection)
  - [ ] Implement debug console with **eruda**
  - [ ] Create variable inspector with **react-devtools**
  - [ ] Build state viewer with **zustand/devtools**
  - [ ] Implement time controls with **react-use-time-travel**
  - [ ] Create replay system with **rrweb**
  - [ ] Build save states with **json-diff**
  - [ ] Implement cheats with **react-cheat-sheet**
  - [ ] Create god mode with debug flags
  - [ ] Build test scenarios with **storybook**
  - [ ] Implement validation with **zod** runtime checks

## Phase 10: Extended Features

### Modding Support (Lua primary with mlua)
- [ ] **Mod System Architecture** (Rust + Lua)
  - [ ] Mod loader with **mlua::Lua** contexts
  - [ ] Mod manager with **toml** manifests
  - [ ] Dependency resolution with **petgraph** DAG
  - [ ] Version checking with **semver** crate
  - [ ] Conflict detection with **indexmap** ordering
  - [ ] Load order with **topological-sort**
  - [ ] Mod packaging with **zip** crate
  - [ ] Distribution with **reqwest** downloading
  - [ ] Workshop integration with Steam API
  - [ ] Documentation with **rustdoc** + Lua docs

- [ ] **Lua Modding API** (mlua bindings)
  - [ ] Expose entities with **mlua::UserData**
  - [ ] Event hooks with **mlua::Function** callbacks
  - [ ] Data modification with **mlua::Table**
  - [ ] UI extension with Lua React bindings
  - [ ] Custom content with **rlua::prelude**
  - [ ] Balance tweaks with Lua hot-reload
  - [ ] New mechanics with **mlua::Scope**
  - [ ] Scenario tools with **lua-cjson**
  - [ ] Debug aids with **mobdebug** remote
  - [ ] Hot reload with **notify** file watcher

- [ ] **Lua Script Libraries**
  - [ ] Game API with **penlight** utilities
  - [ ] Math helpers with **lua-matrix**
  - [ ] Collections with **moses** functional
  - [ ] OOP support with **middleclass**
  - [ ] State machines with **statemachine**
  - [ ] Behavior trees with **behavior3**
  - [ ] Pattern matching with **lpeg**
  - [ ] Testing with **busted** framework
  - [ ] Profiling with **luatrace**
  - [ ] Documentation with **ldoc** generator

### Platform Features
- [ ] **Steam Integration** (Optional)
  - [ ] Implement achievements
  - [ ] Create cloud saves
  - [ ] Build workshop support
  - [ ] Implement leaderboards
  - [ ] Create rich presence
  - [ ] Build overlay support
  - [ ] Implement screenshots
  - [ ] Create broadcasting
  - [ ] Build trading cards
  - [ ] Implement DLC support

- [ ] **System Integration**
  - [ ] Create file associations
  - [ ] Implement clipboard support
  - [ ] Build drag-and-drop
  - [ ] Create system tray
  - [ ] Implement notifications
  - [ ] Build auto-update
  - [ ] Create crash reporting
  - [ ] Implement telemetry (optional)
  - [ ] Build discord presence
  - [ ] Create streaming mode

### Accessibility
- [ ] **Visual Accessibility**
  - [ ] Implement colorblind modes
  - [ ] Create high contrast
  - [ ] Build font scaling
  - [ ] Implement UI scaling
  - [ ] Create screen reader support
  - [ ] Build visual indicators
  - [ ] Implement subtitles
  - [ ] Create motion reduction
  - [ ] Build flash reduction
  - [ ] Implement focus indicators

- [ ] **Control Accessibility**
  - [ ] Create remappable keys
  - [ ] Implement one-handed mode
  - [ ] Build mouse-only mode
  - [ ] Create keyboard-only mode
  - [ ] Implement sticky keys
  - [ ] Build slow keys
  - [ ] Create toggle options
  - [ ] Implement auto-pause
  - [ ] Build difficulty options
  - [ ] Create assist modes

### Quality Assurance
- [ ] **Testing Infrastructure** (Multi-language)
  - [ ] Rust unit tests with **cargo test** + **rstest**
  - [ ] Rust integration with **serial_test** for isolation
  - [ ] Rust property tests with **proptest** + **quickcheck**
  - [ ] Zig tests with **ztest** framework
  - [ ] Lua tests with **busted** BDD framework
  - [ ] Performance tests with **criterion** benchmarks
  - [ ] Stress tests with **drill** load testing
  - [ ] Visual regression with **percy** (frontend)
  - [ ] Test coverage: Rust **tarpaulin**, Lua **luacov**
  - [ ] CI/CD with **cargo-make** + GitHub Actions

- [ ] **Game Balance** (Analytics + Testing)
  - [ ] Balance testing with **monte-carlo** crate
  - [ ] AI vs AI with Rust async simulations
  - [ ] Statistical analysis with **statrs** crate
  - [ ] Playtesting with **tracing** + **console_subscriber**
  - [ ] Feedback with **sentry** Rust SDK
  - [ ] Telemetry with **metrics** crate (local)
  - [ ] A/B testing with **statsig** SDK
  - [ ] Balance reports with **plotters** graphs
  - [ ] Tuning with **egui** debug UI
  - [ ] Live balancing with Lua **hot-reload**

- [ ] **Debugging Tools** (Multi-language)
  - [ ] Rust debugging with **tracing-subscriber**
  - [ ] Zig debugging with built-in test framework
  - [ ] Lua debugging with **mobdebug** + **ZeroBrane**
  - [ ] Memory profiling with **dhat** + **tracy**
  - [ ] Performance profiling with **puffin** + **optick**
  - [ ] Concurrency debugging with **loom**
  - [ ] State inspection with **ron** pretty printing
  - [ ] Network debugging with **rshark** packet capture
  - [ ] Replay debugging with **rr** record/replay
  - [ ] Cross-language with **gdbgui** web interface

## Architecture Notes

### Where to Use Each Language:

**Rust (Core Systems)**
- Game state management
- ECS implementation  
- Save/load system
- Networking layer
- Resource management
- Core game logic
- Platform integration

**Zig (Performance Critical)**
- SIMD math operations
- Frustum culling
- Physics calculations
- Pathfinding algorithms
- Combat calculations
- Influence spread
- Batch processing
- Memory-intensive operations

**Lua (Moddable Content)**
- Game rules and formulas
- AI personalities
- Economic balance
- Event scripting
- Policy effects
- Building definitions
- Unit statistics
- Technology trees
- Cultural traits
- Religious beliefs

**TypeScript (Frontend)**
- UI components
- Rendering pipeline
- Input handling
- Audio management
- Animation system
- Visual effects
- User preferences
- Client-side prediction

## Key JavaScript/TypeScript Libraries by Category:

### 3D Rendering & Graphics
- **three.js**: Core 3D engine
- **@react-three/fiber**: React renderer for Three.js
- **@react-three/drei**: Helper components
- **@react-three/postprocessing**: Effects pipeline
- **deck.gl**: Large-scale data visualization
- **pixi.js**: 2D rendering fallback

### State Management
- **zustand**: Primary app state
- **valtio**: Proxy-based reactivity
- **immer**: Immutable updates
- **@tanstack/react-query**: Server state

### Animation
- **framer-motion**: UI animations
- **react-spring**: Physics animations
- **lottie-react**: Vector animations
- **auto-animate**: Automatic transitions
- **animejs**: Complex sequences

### Data Visualization
- **d3**: Low-level visualization
- **visx**: React + D3 components
- **recharts**: Chart components
- **nivo**: Advanced dataviz
- **react-force-graph-3d**: Network graphs
- **cytoscape**: Graph theory viz

### UI Components
- **@radix-ui**: Headless components
- **@tanstack/react-table**: Data tables
- **react-beautiful-dnd**: Drag and drop
- **react-select**: Advanced selects
- **react-hook-form**: Form management

### Audio
- **tone.js**: Music synthesis
- **howler.js**: Audio playback
- **wavesurfer.js**: Waveform viz
- **pizzicato.js**: Sound effects

### Performance
- **comlink**: Web Worker RPC
- **workerpool**: Worker management
- **fflate**: Fast compression
- **gpu.js**: GPU computation
- **threads.js**: Threading library

### Development
- **vite**: Build tool
- **vitest**: Testing framework
- **playwright**: E2E testing
- **storybook**: Component development
- **zod**: Runtime validation

## Key Rust Libraries by Category:

### Core Systems
- **bevy_ecs**: Entity Component System
- **tokio**: Async runtime
- **serde**: Serialization
- **tauri**: Desktop framework

### Data Structures
- **petgraph**: Graph algorithms
- **ndarray**: N-dimensional arrays
- **dashmap**: Concurrent hashmaps
- **slotmap**: Generational indices

### Math & Geometry
- **nalgebra**: Linear algebra
- **glam**: Fast game math
- **parry2d**: Collision detection
- **euclid**: Type-safe geometry

### Procedural Generation
- **noise**: Noise functions
- **bracket-noise**: Game-oriented noise
- **simdnoise**: SIMD-optimized noise
- **fast_poisson**: Poisson disk sampling

### Pathfinding
- **pathfinding**: A* and more
- **hierarchical_pathfinding**: HPA*
- **rstar**: R-tree spatial index

### Parallel Processing
- **rayon**: Data parallelism
- **crossbeam**: Concurrency tools
- **parking_lot**: Better mutexes
- **tokio**: Async tasks

### Compression & Serialization
- **bincode**: Binary serialization
- **rmp-serde**: MessagePack
- **zstd**: Zstandard compression
- **rkyv**: Zero-copy deserialization

### AI & Logic
- **behavior-tree**: Behavior trees
- **goap**: Goal-oriented planning
- **utility-ai**: Utility-based AI

### Memory Management
- **bumpalo**: Bump allocation
- **typed-arena**: Arena allocator
- **generational-arena**: Safe indices

### Profiling & Debug
- **tracy-client**: Tracy profiler
- **puffin**: Profiling library
- **tracing**: Structured logging
- **criterion**: Benchmarking

## Key Zig Libraries by Category:

### Math & SIMD
- **zmath**: SIMD math library
- **zalgebra**: Linear algebra
- **zlm**: Lightweight math

### Data Structures
- **ziglang-data-structures**: Collections
- **zalloc**: Custom allocators

### Algorithms
- **znoise**: Noise generation
- **zstd**: Compression bindings

### Testing
- **ztest**: Testing framework

## Key Lua Libraries by Category:

### Core Utilities
- **penlight**: Comprehensive utils
- **middleclass**: OOP support
- **moses**: Functional programming
- **lume**: Game utilities

### Data Handling
- **lua-cjson**: JSON parsing
- **msgpack**: Binary serialization
- **lyaml**: YAML support
- **serpent**: Pretty printing

### Game Logic
- **behavior3**: Behavior trees
- **statemachine**: FSM support
- **lpeg**: Pattern matching

### Math & Algorithms
- **lua-matrix**: Matrix math
- **random/lrandom**: RNG
- **perlin**: Noise generation

### Testing & Debug
- **busted**: BDD testing
- **mobdebug**: Remote debugger
- **inspect**: Data inspection
- **luacov**: Code coverage

### Performance Targets:
- 60 FPS with 1M+ tiles visible
- <100MB RAM for core systems
- <1 second save/load time
- Instant UI response (<16ms)
- Support for 100+ AI civs
- 10,000+ units simultaneously
- Sub-second turn processing