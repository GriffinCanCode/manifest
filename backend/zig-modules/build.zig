const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Create a shared library (which we can link as static later)
    const lib = b.addSharedLibrary(.{
        .name = "manifest_zig",
        .root_source_file = b.path("src/lib.zig"),
        .target = target,
        .optimize = optimize,
    });

    lib.linkLibC();

    // Install the library
    b.installArtifact(lib);

    // Simple test step that just builds
    const test_step = b.step("test", "Build and test");
    test_step.dependOn(&lib.step);
}
