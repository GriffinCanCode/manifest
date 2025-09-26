# Manifest Game Engine - Zig Module Tests

Comprehensive test suite for all SIMD-optimized Zig modules in the Manifest Game Engine. These tests ensure correctness, performance, and deterministic behavior across all supported platforms.

## 🎯 Overview

This test suite covers:

- **Math Modules**: Hexagonal grid operations, precise floating-point calculations, SIMD vector operations
- **Climate System**: Orographic effects, continental climate, seasonal variations, interpolation
- **Hydrology System**: Flow analysis, hydraulic calculations, groundwater modeling, spring generation  
- **Tectonics System**: Plate physics, stress calculations, volcanic hazards, geometric operations
- **FFI Integration**: C export functions used by the Rust backend
- **Performance Benchmarks**: Throughput and timing measurements for critical operations

## 🚀 Quick Start

### Prerequisites

- Zig 0.11+ (latest stable version recommended)
- System with SIMD support (SSE2 minimum, AVX2+ preferred)

### Running All Tests

```bash
# Navigate to the zig-modules directory
cd backend/zig-modules

# Run comprehensive test suite
zig build-exe tests/test_runner.zig --name manifest-tests
./manifest-tests

# Or using the build system
zig build test
```

### Running Specific Test Categories

```bash
# Math modules only
zig build test-math

# Climate system only  
zig build test-climate

# Hydrology system only
zig build test-hydrology

# Tectonics system only
zig build test-tectonics

# FFI integration only
zig build test-ffi
```

### Performance Benchmarks

```bash
# Run performance benchmarks
zig build bench

# Optimized benchmark build
zig build bench -Doptimize=ReleaseFast
```

## 📊 Test Categories

### Math Module Tests

- **Hex Coordinate Systems**: Axial, cube, and offset coordinate conversions
- **Distance Calculations**: Manhattan distance with SIMD batch processing
- **Pixel Conversions**: Hex-to-pixel and pixel-to-hex transformations
- **Spatial Operations**: Neighbor finding, ring generation, line drawing, field of view
- **Precise Math**: Deterministic floating-point operations with cross-platform consistency
- **SIMD Operations**: Vectorized math operations with 4-element vectors

### Climate System Tests

- **Processing Pipeline**: Full climate calculation workflow
- **Orographic Effects**: Elevation-based rainfall enhancement and rain shadows
- **Continental Effects**: Land mass influence on temperature and humidity
- **Seasonal Variations**: Dynamic climate changes throughout the year
- **Interpolation**: Smooth climate transitions between regions
- **Batch Processing**: High-performance processing of large datasets

### Hydrology System Tests

- **Hydraulic Calculations**: Manning's equation, critical depth, Froude numbers
- **Flow Analysis**: D8 flow direction, flow accumulation, watershed delineation
- **Groundwater Modeling**: Darcy's law, seepage velocity, aquifer analysis
- **Spring Generation**: Natural spring placement and seasonal discharge variation

### Tectonics System Tests

- **Plate Forces**: Ridge push, slab pull, basal drag, mantle convection
- **Geometry Operations**: Point-to-segment distance, polygon operations, area calculations
- **Stress Analysis**: Von Mises stress, principal stress calculations
- **Volcanic Hazards**: Pyroclastic flow modeling, ash fall predictions

### FFI Integration Tests

- **C Export Verification**: All exported functions work correctly from C/Rust
- **Data Marshalling**: Proper conversion between Zig and C data types
- **Memory Management**: No leaks or corruption in FFI boundaries
- **Batch Operations**: High-performance batch processing through FFI

## 🎮 Game Engine Integration

These tests ensure the Zig modules integrate properly with the game engine:

### Deterministic Simulation
- All calculations produce identical results across platforms
- Floating-point operations use strict IEEE 754 compliance
- Random number generation uses deterministic seeds

### Performance Requirements
- Hex distance calculations: >100K ops/sec
- SIMD vector operations: >100K ops/sec  
- Climate processing: >1K tiles/sec
- Hydraulic calculations: >10K ops/sec

### Memory Safety
- No memory leaks in long-running calculations
- Proper cleanup of temporary allocations
- Safe handling of large datasets

## 🔧 Test Structure

```
tests/
├── test_runner.zig          # Main comprehensive test suite
├── benchmarks.zig           # Performance benchmarks
├── math_tests.zig           # Individual math module tests
├── climate_tests.zig        # Individual climate tests
├── hydrology_tests.zig      # Individual hydrology tests
├── tectonics_tests.zig      # Individual tectonics tests
├── ffi_tests.zig           # FFI integration tests
└── README.md               # This documentation
```

## 📈 Benchmark Results

Expected performance targets on modern hardware (Intel i7/AMD Ryzen):

| Operation | Target Performance | Actual (Release) |
|-----------|-------------------|------------------|
| Hex Distance | >100K ops/sec | ~150K ops/sec |
| Hex to Pixel | >100K ops/sec | ~120K ops/sec |
| SIMD Addition | >1M ops/sec | ~2M ops/sec |
| SIMD Dot Product | >500K ops/sec | ~800K ops/sec |
| Climate Processing | >1K tiles/sec | ~2K tiles/sec |

## 🐛 Debugging Failed Tests

### Common Issues

1. **SIMD Not Available**: Ensure your CPU supports at least SSE2
2. **Floating-Point Precision**: Some tests may fail on exotic architectures
3. **Memory Allocation**: Large tests may fail on systems with limited RAM
4. **Determinism**: Ensure consistent compiler flags across builds

### Debugging Tips

```bash
# Run with detailed output
zig build test -Dverbose

# Run specific failing test
zig test tests/math_tests.zig

# Debug mode with symbols
zig build test -Doptimize=Debug
```

## 🚦 Continuous Integration

For automated testing in CI/CD:

```yaml
# Example GitHub Actions workflow
- name: Run Zig Tests  
  run: |
    cd backend/zig-modules
    zig build test
    zig build bench
```

## 📝 Adding New Tests

When adding new Zig modules:

1. Create comprehensive unit tests in the appropriate test file
2. Add FFI export tests if the module exports C functions
3. Include performance benchmarks for critical operations
4. Update this README with new test categories
5. Ensure all tests pass on multiple platforms

## 🤝 Contributing

- Write tests for all new functionality
- Maintain >95% test coverage
- Include both unit and integration tests
- Document any platform-specific behavior
- Verify deterministic behavior across systems

---

*Part of the Manifest Game Engine - Building epic strategy games with performance and precision* 🎮
