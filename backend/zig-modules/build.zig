const std = @import("std");

/// ManifestRustTS Zig Math & SIMD Library Build Configuration
///
/// This build script creates optimized mathematical and SIMD operations for the
/// Manifest game engine, providing cross-platform deterministic computations
/// with high-performance vector operations through FFI to Rust.
///
/// COMPATIBILITY: Designed for Zig 0.15.1+
/// TARGETS: All platforms supported by Zig (cross-compilation ready)
/// OUTPUT: Object file + Static archive for Rust linking
///
/// Key Features:
/// - Deterministic floating-point operations for cross-platform consistency
/// - SIMD-accelerated vector mathematics
/// - Hexagonal grid calculations for tile-based game mechanics
/// - C bridge for maximum compatibility with Rust FFI
///
/// Generated Artifacts:
/// - libmanifest_zig.o  -> Object file for direct linking
/// - libmanifest_zig.a  -> Static archive for Cargo integration (recommended)
pub fn build(b: *std.Build) void {
    // ============================================================================
    // BUILD CONFIGURATION
    // ============================================================================

    // Cross-compilation target selection
    // Supports all Zig targets: x86_64, aarch64, wasm32, etc.
    // Override with: zig build -Dtarget=x86_64-windows
    const target = b.standardTargetOptions(.{});

    // Optimization mode selection
    // - Debug: Full debug info, safety checks, no optimization
    // - ReleaseSafe: Optimized + safety checks (recommended for production)
    // - ReleaseFast: Maximum performance, minimal safety
    // - ReleaseSmall: Size-optimized build
    // Override with: zig build -Doptimize=ReleaseFast
    const optimize = b.standardOptimizeOption(.{});

    // ============================================================================
    // LIBRARY MODULE SETUP
    // ============================================================================

    // Primary library module for Zig-to-Zig imports
    // This allows other Zig projects to import our math/SIMD functions directly
    // Usage in other Zig code: @import("manifest_zig")
    const lib_module = b.addModule("manifest_zig", .{
        .root_source_file = b.path("src/lib.zig"),
        .target = target,
    });

    // Main library object for FFI with Rust
    // Creates a relocatable object file that Rust can link against
    //
    // IMPORTANT: Object files (.o) are the core artifact for Rust FFI.
    // The static archive (.a) is generated for convenience but both work.
    const lib = b.addObject(.{
        .name = "libmanifest_zig",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/lib.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });

    // Add C bridge for SIMD operations
    // This C file contains optimized SIMD intrinsics that may not be directly
    // expressible in Zig, providing maximum performance for critical paths.
    //
    // C Compiler flags:
    // -std=c99: Use C99 standard for maximum compatibility
    // -O3: Maximum optimization for performance-critical SIMD code
    lib.addCSourceFile(.{ .file = b.path("src/simd_bridge.c"), .flags = &.{ "-std=c99", "-O3" } });

    // Link against system libc for C bridge functionality
    // Required for: malloc, math functions, SIMD intrinsics
    lib.linkLibC();

    // ============================================================================
    // ARTIFACT INSTALLATION & PACKAGING
    // ============================================================================

    // Install object file to lib/ directory
    //
    // NOTE: Object files don't have standard Zig installation procedures,
    // so we manually specify the destination directory. This creates:
    // zig-out/lib/libmanifest_zig.o
    const lib_install = b.addInstallArtifact(lib, .{
        .dest_dir = .{ .override = .{ .custom = "lib" } },
    });

    // Create static archive (.a file) for enhanced Rust integration
    //
    // Archives are often preferred by build systems like Cargo because:
    // 1. They can contain multiple object files if needed
    // 2. Better toolchain compatibility
    // 3. Easier dependency management
    // 4. Standard format across all platforms
    const archive_step = b.step("archive", "Create static archive for Rust linking");
    const create_archive = b.addSystemCommand(&.{ "ar", "rcs" });
    create_archive.addArg(b.getInstallPath(.lib, "libmanifest_zig.a"));
    create_archive.addArtifactArg(lib);
    create_archive.step.dependOn(&lib_install.step);
    archive_step.dependOn(&create_archive.step);

    // ============================================================================
    // TESTING INFRASTRUCTURE
    // ============================================================================

    // Comprehensive test suite for all library functions
    // Tests deterministic math, SIMD operations, hex grid calculations
    // and cross-platform consistency validations
    const lib_tests = b.addTest(.{
        .root_module = lib_module,
    });
    lib_tests.linkLibC(); // Required for C bridge testing

    const run_lib_tests = b.addRunArtifact(lib_tests);

    // Primary test runner step
    // Usage: zig build test
    const test_step = b.step("test", "Run comprehensive library test suite");
    test_step.dependOn(&run_lib_tests.step);

    // ============================================================================
    // BUILD STEPS & WORKFLOW
    // ============================================================================

    // Create object file only (for direct Rust integration)
    // Usage: zig build lib
    const build_lib_step = b.step("lib", "Build object file for Rust FFI");
    build_lib_step.dependOn(&lib_install.step);

    // DEFAULT BUILD TARGET: Complete library with both formats
    // Usage: zig build
    // Creates both libmanifest_zig.o and libmanifest_zig.a
    b.getInstallStep().dependOn(&lib_install.step);

    // Auto-generate static archive on default build
    // This ensures Rust integration works out-of-the-box
    const create_archive_default = b.addSystemCommand(&.{ "ar", "rcs" });
    create_archive_default.addArg(b.getInstallPath(.lib, "libmanifest_zig.a"));
    create_archive_default.addArtifactArg(lib);
    create_archive_default.step.dependOn(&lib_install.step);
    b.getInstallStep().dependOn(&create_archive_default.step);

    // ============================================================================
    // DEVELOPMENT & MAINTENANCE UTILITIES
    // ============================================================================

    // Syntax and semantic checking without full compilation
    // Useful for CI/CD pipelines and fast development feedback
    // Usage: zig build check
    const check_step = b.step("check", "Validate code compiles without building artifacts");
    check_step.dependOn(&lib.step);

    // Convenience alias for test execution
    // Usage: zig build run-tests
    const run_tests_step = b.step("run-tests", "Execute all test suites");
    run_tests_step.dependOn(&run_lib_tests.step);

    // Clean all generated build artifacts
    // Usage: zig build clean
    // Removes: zig-out/, .zig-cache/ contents
    const clean_step = b.step("clean", "Remove all build artifacts");
    const clean_cmd = b.addRemoveDirTree(b.path("zig-out"));
    clean_step.dependOn(&clean_cmd.step);

    // ============================================================================
    // FUTURE-PROOFING NOTES
    // ============================================================================
    //
    // 1. VERSION COMPATIBILITY:
    //    - Built for Zig 0.15.1+ API
    //    - If Zig changes addObject() API, check Step.Compile documentation
    //    - Monitor .zig-cache behavior changes in future versions
    //
    // 2. TARGET EXPANSION:
    //    - Add conditional compilation for platform-specific SIMD
    //    - Consider GPU compute shaders for future versions
    //    - WebAssembly SIMD may require special handling
    //
    // 3. RUST FFI EVOLUTION:
    //    - bindgen integration may require header file generation
    //    - Consider cbindgen for reverse FFI (Rust->Zig callbacks)
    //    - Monitor Rust's FFI stability guarantees
    //
    // 4. BUILD SYSTEM INTEGRATION:
    //    - CMake integration possible via ExternalProject
    //    - Cargo workspace integration in build.rs
    //    - CI/CD artifact caching for cross-compilation
    //
    // 5. PERFORMANCE MONITORING:
    //    - Add benchmark step for regression detection
    //    - Profile-guided optimization integration
    //    - Cross-platform performance validation
    //
}
