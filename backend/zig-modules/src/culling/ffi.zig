//! FFI exports for culling system
//!
//! C-compatible interface for Rust integration of the frustum culling,
//! LOD management, and occlusion culling systems.

const std = @import("std");

const hex = @import("../math/hex.zig");
const culling = @import("mod.zig");

// Frustum culling FFI exports
export fn manifest_frustum_from_matrix(view_projection: [*]const f32, frustum_out: *culling.Frustum) void {
    const matrix: [16]f32 = view_projection[0..16].*;
    frustum_out.* = culling.Frustum.fromMatrix(matrix);
}

export fn manifest_frustum_test_point(frustum: *const culling.Frustum, x: f32, y: f32, z: f32) bool {
    const point = culling.Vec3.init(x, y, z);
    return frustum.testPoint(point);
}

export fn manifest_frustum_test_sphere(frustum: *const culling.Frustum, center_x: f32, center_y: f32, center_z: f32, radius: f32) u8 {
    const center = culling.Vec3.init(center_x, center_y, center_z);
    const result = frustum.testSphere(center, radius);
    return @intFromEnum(result);
}

export fn manifest_frustum_test_aabb(frustum: *const culling.Frustum, min_x: f32, min_y: f32, min_z: f32, max_x: f32, max_y: f32, max_z: f32) u8 {
    const min_point = culling.Vec3.init(min_x, min_y, min_z);
    const max_point = culling.Vec3.init(max_x, max_y, max_z);
    const result = frustum.testAABB(min_point, max_point);
    return @intFromEnum(result);
}

export fn manifest_frustum_batch_test_spheres(frustum: *const culling.Frustum, centers_x: [*]const f32, centers_y: [*]const f32, centers_z: [*]const f32, radii: [*]const f32, count: usize, results: [*]u8) void {
    const max_batch = 256;
    const actual_count = @min(count, max_batch);

    var spheres: [max_batch]culling.Sphere = undefined;
    var test_results: [max_batch]culling.CullingResult = undefined;

    for (0..actual_count) |i| {
        spheres[i] = culling.Sphere.init(culling.Vec3.init(centers_x[i], centers_y[i], centers_z[i]), radii[i]);
    }

    culling.BatchCuller.testSpheres(frustum.*, spheres[0..actual_count], test_results[0..actual_count]);

    for (0..actual_count) |i| {
        results[i] = @intFromEnum(test_results[i]);
    }
}

export fn manifest_frustum_batch_test_aabbs(frustum: *const culling.Frustum, mins_x: [*]const f32, mins_y: [*]const f32, mins_z: [*]const f32, maxs_x: [*]const f32, maxs_y: [*]const f32, maxs_z: [*]const f32, count: usize, results: [*]u8) void {
    const max_batch = 256;
    const actual_count = @min(count, max_batch);

    var aabbs: [max_batch]culling.AABB = undefined;
    var test_results: [max_batch]culling.CullingResult = undefined;

    for (0..actual_count) |i| {
        aabbs[i] = culling.AABB.init(culling.Vec3.init(mins_x[i], mins_y[i], mins_z[i]), culling.Vec3.init(maxs_x[i], maxs_y[i], maxs_z[i]));
    }

    culling.BatchCuller.testAABBs(frustum.*, aabbs[0..actual_count], test_results[0..actual_count]);

    for (0..actual_count) |i| {
        results[i] = @intFromEnum(test_results[i]);
    }
}

// Hex culling FFI exports
export fn manifest_hex_cull_tiles(frustum: *const culling.Frustum, tile_coords_q: [*]const i32, tile_coords_r: [*]const i32, elevations: [*]const f32, tile_size: f32, count: usize, results: [*]u8) void {
    const max_batch = 256;
    const actual_count = @min(count, max_batch);

    var coords: [max_batch]hex.HexCoord = undefined;
    var test_results: [max_batch]culling.CullingResult = undefined;

    for (0..actual_count) |i| {
        coords[i] = hex.HexCoord.init(tile_coords_q[i], tile_coords_r[i]);
    }

    culling.HexCuller.cullHexTiles(frustum.*, coords[0..actual_count], elevations[0..actual_count], tile_size, test_results[0..actual_count]);

    for (0..actual_count) |i| {
        results[i] = @intFromEnum(test_results[i]);
    }
}

// LOD calculation FFI exports
export fn manifest_lod_calculate_single(
    camera_x: f32,
    camera_y: f32,
    camera_z: f32,
    camera_forward_x: f32,
    camera_forward_y: f32,
    camera_forward_z: f32,
    viewport_width: f32,
    viewport_height: f32,
    fov_y: f32,
    object_x: f32,
    object_y: f32,
    object_z: f32,
    object_radius: f32,
    use_screen_size: bool,
    distance_thresholds: [*]const f32, // 5 elements
    screen_size_thresholds: [*]const f32, // 5 elements
    bias: f32,
    level_out: *u8,
    distance_out: *f32,
    screen_size_out: *f32,
    transition_alpha_out: *f32,
) void {
    const calculator = culling.LODCalculator.init(culling.Vec3.init(camera_x, camera_y, camera_z), culling.Vec3.init(camera_forward_x, camera_forward_y, camera_forward_z), viewport_width, viewport_height, fov_y);

    const config = culling.LODConfig{
        .distance_thresholds = distance_thresholds[0..5].*,
        .screen_size_thresholds = screen_size_thresholds[0..5].*,
        .use_screen_size = use_screen_size,
        .bias = bias,
    };

    const object_pos = culling.Vec3.init(object_x, object_y, object_z);
    const result = calculator.calculateLOD(object_pos, object_radius, config);

    level_out.* = @intFromEnum(result.level);
    distance_out.* = result.distance;
    screen_size_out.* = result.screen_size;
    transition_alpha_out.* = result.transition_alpha;
}

export fn manifest_lod_calculate_batch(
    camera_x: f32,
    camera_y: f32,
    camera_z: f32,
    camera_forward_x: f32,
    camera_forward_y: f32,
    camera_forward_z: f32,
    viewport_width: f32,
    viewport_height: f32,
    fov_y: f32,
    objects_x: [*]const f32,
    objects_y: [*]const f32,
    objects_z: [*]const f32,
    radii: [*]const f32,
    use_screen_size: bool,
    distance_thresholds: [*]const f32, // 5 elements
    screen_size_thresholds: [*]const f32, // 5 elements
    bias: f32,
    count: usize,
    levels_out: [*]u8,
    distances_out: [*]f32,
    screen_sizes_out: [*]f32,
    transition_alphas_out: [*]f32,
) void {
    const max_batch = 256;
    const actual_count = @min(count, max_batch);

    const calculator = culling.LODCalculator.init(culling.Vec3.init(camera_x, camera_y, camera_z), culling.Vec3.init(camera_forward_x, camera_forward_y, camera_forward_z), viewport_width, viewport_height, fov_y);

    const config = culling.LODConfig{
        .distance_thresholds = distance_thresholds[0..5].*,
        .screen_size_thresholds = screen_size_thresholds[0..5].*,
        .use_screen_size = use_screen_size,
        .bias = bias,
    };

    var positions: [max_batch]culling.Vec3 = undefined;
    var object_radii: [max_batch]f32 = undefined;
    var results: [max_batch]culling.LODResult = undefined;

    for (0..actual_count) |i| {
        positions[i] = culling.Vec3.init(objects_x[i], objects_y[i], objects_z[i]);
        object_radii[i] = radii[i];
    }

    calculator.calculateLODBatch(positions[0..actual_count], object_radii[0..actual_count], config, results[0..actual_count]);

    for (0..actual_count) |i| {
        levels_out[i] = @intFromEnum(results[i].level);
        distances_out[i] = results[i].distance;
        screen_sizes_out[i] = results[i].screen_size;
        transition_alphas_out[i] = results[i].transition_alpha;
    }
}

export fn manifest_hex_lod_calculate_tiles(camera_x: f32, camera_y: f32, camera_z: f32, camera_forward_x: f32, camera_forward_y: f32, camera_forward_z: f32, viewport_width: f32, viewport_height: f32, fov_y: f32, tile_coords_q: [*]const i32, tile_coords_r: [*]const i32, elevations: [*]const f32, tile_size: f32, count: usize, levels_out: [*]u8, distances_out: [*]f32, screen_sizes_out: [*]f32, transition_alphas_out: [*]f32) void {
    const max_batch = 256;
    const actual_count = @min(count, max_batch);

    const calculator = culling.HexLODCalculator.init(culling.Vec3.init(camera_x, camera_y, camera_z), culling.Vec3.init(camera_forward_x, camera_forward_y, camera_forward_z), viewport_width, viewport_height, fov_y, tile_size);

    var coords: [max_batch]hex.HexCoord = undefined;
    var tile_elevations: [max_batch]f32 = undefined;
    var results: [max_batch]culling.LODResult = undefined;

    for (0..actual_count) |i| {
        coords[i] = hex.HexCoord.init(tile_coords_q[i], tile_coords_r[i]);
        tile_elevations[i] = elevations[i];
    }

    calculator.calculateHexTileLODs(coords[0..actual_count], tile_elevations[0..actual_count], results[0..actual_count]);

    for (0..actual_count) |i| {
        levels_out[i] = @intFromEnum(results[i].level);
        distances_out[i] = results[i].distance;
        screen_sizes_out[i] = results[i].screen_size;
        transition_alphas_out[i] = results[i].transition_alpha;
    }
}

// Culling statistics FFI exports
export fn manifest_culling_stats_create() *culling.CullingStats {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();
    const stats = allocator.create(culling.CullingStats) catch @panic("Failed to allocate CullingStats");
    stats.* = culling.CullingStats.init();
    return stats;
}

export fn manifest_culling_stats_destroy(stats: *culling.CullingStats) void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();
    allocator.destroy(stats);
}

export fn manifest_culling_stats_reset(stats: *culling.CullingStats) void {
    stats.reset();
}

export fn manifest_culling_stats_add_result(stats: *culling.CullingStats, result: u8) void {
    const culling_result: culling.CullingResult = @enumFromInt(result);
    stats.addResult(culling_result);
}

export fn manifest_culling_stats_get_ratio(stats: *const culling.CullingStats) f32 {
    return stats.getCullingRatio();
}

export fn manifest_culling_stats_get_counts(stats: *const culling.CullingStats, total: *u32, culled: *u32, visible: *u32, intersecting: *u32) void {
    total.* = stats.total_tested;
    culled.* = stats.culled_objects;
    visible.* = stats.visible_objects;
    intersecting.* = stats.intersecting_objects;
}
