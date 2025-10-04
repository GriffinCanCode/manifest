![Manifest Logo](asssets/manifest.png)

# Manifest: Grand Strategy Empire Builder

**Build civilizations. Shape history. Command the future.**

A strategy game that simulates entire civilizations from the dawn of agriculture to the space age. Every citizen, every trade route, every diplomatic negotiation matters in this living world of procedural history.

## The Vision

A strategy game that models interconnected systems. Economic pressures can spark political changes, cultural movements shift territorial boundaries, and technological advances affect multiple aspects of society. The focus extends beyond military conquest to include the broader development of civilizations.

## Technical Foundation

Built using modern systems programming languages for reliable simulation:

**Rust Core Engine**
- Deterministic ECS architecture using Bevy for consistent simulation results
- Multi-threaded processing with Rayon for parallel computation
- Procedural generation systems creating varied game worlds
- AI systems with Lua-scriptable personalities

**Performance Optimization**
- Zig integration with SIMD optimization for mathematical operations
- Spatial indexing supporting large numbers of game entities
- Efficient pathfinding algorithms for real-time calculations
- Level-of-detail systems scaling from citizens to empires

**Desktop Application**
- Tauri 2.0 framework combining native performance with web technologies
- React + Three.js frontend for 3D visualization
- WebGL2/WebGPU rendering pipeline for hex-based world maps
- Modding support through Lua scripting and hot-reload

## Gameplay Systems

**Economics**: Supply chains react to political events, disasters, and technological changes. Markets respond when trade routes are disrupted, then adapt through alternative pathways.

**Politics**: Government systems evolve from tribal councils to various forms of governance. Interest groups form, compete, and can challenge established authority based on societal conditions.

**Culture**: Beliefs, art, and social structures spread across populations, creating cultural regions that shift over time.

**AI Opponents**: Computer civilizations use goal-oriented planning and personality systems. Leaders remember past interactions and pursue objectives based on their characteristics.

## Technical Features

- **Scale**: Support for large continental maps with hexagonal tile systems
- **Time Spans**: Gameplay across multiple technological eras
- **Deterministic**: Identical starting conditions produce identical results
- **Modding**: Lua scripting for game modifications
- **Cross-Platform**: Windows, macOS, and Linux support
- **Performance**: Designed to maintain smooth framerates during gameplay

## Development Approach

Systems focus on emergent gameplay rather than scripted events. Economic models use supply-demand calculations. Political systems draw from historical government structures. Combat includes factors like logistics, morale, and terrain.

The goal is a strategy game where decisions have long-term consequences and success comes from understanding interconnected systems.

## Current Status

**In Active Development**
- Core simulation engine: Implemented
- Hex-based world generation: Complete with Zig SIMD optimization
- ECS architecture: Implemented with hot-reload capabilities
- Procedural content systems: Lua integration in progress
- 3D rendering pipeline: Three.js with WebGL2 support

*A grand strategy game built with modern programming languages and game development practices.*

---

**Architecture**: Rust + Zig + Lua + TypeScript  
**Target Platforms**: Windows, macOS, Linux  
**Engine**: Custom ECS with Tauri 2.0  
**License**: [TBD]  
**Contact**: [GriffinCanCode@gmail.com]
