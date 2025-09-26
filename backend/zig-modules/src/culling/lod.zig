//! Level of Detail (LOD) system for distance-based quality adjustment
//!
//! Provides automatic LOD selection based on camera distance, screen size,
//! and performance requirements with SIMD batch processing.

const std = @import("std");

const hex = @import("../math/hex.zig");
const precise = @import("../math/precise.zig");
const simd = @import("../simd/simd.zig");
const frustum = @import("frustum.zig");

/// LOD level specification
pub const LODLevel = enum(u8) {
    highest = 0, // Closest, highest detail
    high = 1, // Close, high detail
    medium = 2, // Medium distance, medium detail
    low = 3, // Far, low detail
    lowest = 4, // Farthest, lowest detail/billboard
};

/// LOD configuration for different object types
pub const LODConfig = struct {
    /// Distance thresholds for each LOD level (squared distances for performance)
    distance_thresholds: [5]f32,

    /// Screen size thresholds in pixels
    screen_size_thresholds: [5]f32,

    /// Whether to use screen size vs distance
    use_screen_size: bool,

    /// Bias factor for LOD selection (-1.0 to 1.0)
    bias: f32,

    pub fn default() LODConfig {
        return LODConfig{
            .distance_thresholds = [5]f32{ 0.0, 100.0, 400.0, 1600.0, 6400.0 }, // 0, 10, 20, 40, 80 units
            .screen_size_thresholds = [5]f32{ 1000.0, 200.0, 50.0, 10.0, 1.0 },
            .use_screen_size = false,
            .bias = 0.0,
        };
    }

    pub fn forTerrain() LODConfig {
        return LODConfig{
            .distance_thresholds = [5]f32{ 0.0, 64.0, 256.0, 1024.0, 4096.0 },
            .screen_size_thresholds = [5]f32{ 500.0, 100.0, 25.0, 5.0, 1.0 },
            .use_screen_size = true,
            .bias = -0.2, // Prefer higher detail for terrain
        };
    }

    pub fn forUnits() LODConfig {
        return LODConfig{
            .distance_thresholds = [5]f32{ 0.0, 25.0, 100.0, 400.0, 1600.0 },
            .screen_size_thresholds = [5]f32{ 200.0, 50.0, 15.0, 5.0, 1.0 },
            .use_screen_size = true,
            .bias = 0.1, // Slightly prefer lower detail for units
        };
    }

    pub fn forBuildings() LODConfig {
        return LODConfig{
            .distance_thresholds = [5]f32{ 0.0, 144.0, 576.0, 2304.0, 9216.0 },
            .screen_size_thresholds = [5]f32{ 800.0, 150.0, 40.0, 10.0, 2.0 },
            .use_screen_size = true,
            .bias = 0.0,
        };
    }
};

/// LOD selection result with additional metadata
pub const LODResult = struct {
    level: LODLevel,
    distance: f32,
    screen_size: f32,
    transition_alpha: f32, // For smooth LOD transitions (0.0 - 1.0)

    pub fn init(level: LODLevel) LODResult {
        return LODResult{
            .level = level,
            .distance = 0.0,
            .screen_size = 0.0,
            .transition_alpha = 1.0,
        };
    }
};

/// Main LOD calculator with SIMD optimization
pub const LODCalculator = struct {
    const Self = @This();

    camera_position: frustum.Vec3,
    camera_forward: frustum.Vec3,
    viewport_width: f32,
    viewport_height: f32,
    fov_y: f32,
    performance_scale: f32, // 0.5 - 2.0 for performance adjustment

    pub fn init(camera_position: frustum.Vec3, camera_forward: frustum.Vec3, viewport_width: f32, viewport_height: f32, fov_y: f32) Self {
        return Self{
            .camera_position = camera_position,
            .camera_forward = camera_forward,
            .viewport_width = viewport_width,
            .viewport_height = viewport_height,
            .fov_y = fov_y,
            .performance_scale = 1.0,
        };
    }

    pub fn updateCamera(self: *Self, position: frustum.Vec3, forward: frustum.Vec3) void {
        self.camera_position = position;
        self.camera_forward = forward;
    }

    pub fn setPerformanceScale(self: *Self, scale: f32) void {
        self.performance_scale = @max(0.1, @min(3.0, scale));
    }

    /// Calculate LOD for single object
    pub fn calculateLOD(self: Self, object_position: frustum.Vec3, object_radius: f32, config: LODConfig) LODResult {
        const distance_sq = self.calculateDistanceSquared(object_position);
        const distance = std.math.sqrt(distance_sq);

        var result = LODResult.init(.highest);
        result.distance = distance;

        if (config.use_screen_size) {
            result.screen_size = self.calculateScreenSize(object_position, object_radius);
            result.level = self.selectLODByScreenSize(result.screen_size, config);
        } else {
            result.level = self.selectLODByDistance(distance_sq, config);
        }

        // Apply bias and performance scaling
        result.level = self.applyBiasAndScale(result.level, config.bias);

        // Calculate transition alpha for smooth LOD changes
        result.transition_alpha = self.calculateTransitionAlpha(result.level, distance_sq, result.screen_size, config);

        return result;
    }

    /// Batch calculate LODs with SIMD optimization
    pub fn calculateLODBatch(self: Self, positions: []const frustum.Vec3, radii: []const f32, config: LODConfig, results: []LODResult) void {
        std.debug.assert(positions.len == radii.len);
        std.debug.assert(positions.len == results.len);

        const camera_pos_simd = self.camera_position.toSimdVec();

        var i: usize = 0;
        while (i + 3 < positions.len) : (i += 4) {
            // Pack 4 positions into SIMD vectors
            const pos_x = [4]f32{ positions[i].x, positions[i + 1].x, positions[i + 2].x, positions[i + 3].x };
            const pos_y = [4]f32{ positions[i].y, positions[i + 1].y, positions[i + 2].y, positions[i + 3].y };
            const pos_z = [4]f32{ positions[i].z, positions[i + 1].z, positions[i + 2].z, positions[i + 3].z };

            // Calculate distance vectors using SIMD
            const delta_x = simd.subVec4(pos_x, [4]f32{ camera_pos_simd[0], camera_pos_simd[0], camera_pos_simd[0], camera_pos_simd[0] });
            const delta_y = simd.subVec4(pos_y, [4]f32{ camera_pos_simd[1], camera_pos_simd[1], camera_pos_simd[1], camera_pos_simd[1] });
            const delta_z = simd.subVec4(pos_z, [4]f32{ camera_pos_simd[2], camera_pos_simd[2], camera_pos_simd[2], camera_pos_simd[2] });

            // Calculate squared distances
            const dist_sq = simd.addVec4(simd.addVec4(simd.mulVec4(delta_x, delta_x), simd.mulVec4(delta_y, delta_y)), simd.mulVec4(delta_z, delta_z));

            // Process each of the 4 results
            for (0..4) |j| {
                if (i + j < positions.len) {
                    const distance = std.math.sqrt(dist_sq[j]);
                    results[i + j].distance = distance;

                    if (config.use_screen_size) {
                        results[i + j].screen_size = self.calculateScreenSize(positions[i + j], radii[i + j]);
                        results[i + j].level = self.selectLODByScreenSize(results[i + j].screen_size, config);
                    } else {
                        results[i + j].level = self.selectLODByDistance(dist_sq[j], config);
                    }

                    // Apply bias and calculate transition
                    results[i + j].level = self.applyBiasAndScale(results[i + j].level, config.bias);
                    results[i + j].transition_alpha = self.calculateTransitionAlpha(results[i + j].level, dist_sq[j], results[i + j].screen_size, config);
                }
            }
        }

        // Handle remaining objects
        while (i < positions.len) : (i += 1) {
            results[i] = self.calculateLOD(positions[i], radii[i], config);
        }
    }

    fn calculateDistanceSquared(self: Self, position: frustum.Vec3) f32 {
        const delta_x = position.x - self.camera_position.x;
        const delta_y = position.y - self.camera_position.y;
        const delta_z = position.z - self.camera_position.z;

        return precise.detAdd(precise.detAdd(precise.detMul(delta_x, delta_x), precise.detMul(delta_y, delta_y)), precise.detMul(delta_z, delta_z));
    }

    fn calculateScreenSize(self: Self, position: frustum.Vec3, radius: f32) f32 {
        const distance = std.math.sqrt(self.calculateDistanceSquared(position));
        if (distance < 0.001) return 1000.0; // Very close, assume large

        // Calculate projected radius using perspective projection
        const half_fov = self.fov_y * 0.5;
        const projected_radius = radius / (distance * std.math.tan(half_fov));

        return projected_radius * self.viewport_height * 0.5;
    }

    fn selectLODByDistance(self: Self, distance_sq: f32, config: LODConfig) LODLevel {
        const scaled_distance_sq = distance_sq / (self.performance_scale * self.performance_scale);

        if (scaled_distance_sq <= config.distance_thresholds[0]) return .highest;
        if (scaled_distance_sq <= config.distance_thresholds[1]) return .high;
        if (scaled_distance_sq <= config.distance_thresholds[2]) return .medium;
        if (scaled_distance_sq <= config.distance_thresholds[3]) return .low;
        return .lowest;
    }

    fn selectLODByScreenSize(self: Self, screen_size: f32, config: LODConfig) LODLevel {
        const scaled_size = screen_size * self.performance_scale;

        if (scaled_size >= config.screen_size_thresholds[0]) return .highest;
        if (scaled_size >= config.screen_size_thresholds[1]) return .high;
        if (scaled_size >= config.screen_size_thresholds[2]) return .medium;
        if (scaled_size >= config.screen_size_thresholds[3]) return .low;
        return .lowest;
    }

    fn applyBiasAndScale(self: Self, level: LODLevel, bias: f32) LODLevel {
        if (bias == 0.0) return level;

        const level_value = @as(f32, @floatFromInt(@intFromEnum(level)));
        const biased_value = level_value + (bias * self.performance_scale);
        const clamped_value = @max(0.0, @min(4.0, biased_value));

        return @enumFromInt(@as(u8, @intFromFloat(clamped_value)));
    }

    fn calculateTransitionAlpha(self: Self, level: LODLevel, distance_sq: f32, screen_size: f32, config: LODConfig) f32 {
        _ = self;

        const level_index = @intFromEnum(level);
        if (level_index == 0) return 1.0; // Highest detail, no transition

        const prev_threshold = if (config.use_screen_size)
            config.screen_size_thresholds[level_index - 1]
        else
            config.distance_thresholds[level_index - 1];

        const current_threshold = if (config.use_screen_size)
            config.screen_size_thresholds[level_index]
        else
            config.distance_thresholds[level_index];

        const test_value = if (config.use_screen_size) screen_size else distance_sq;

        // Calculate alpha for smooth transition between LOD levels
        if (config.use_screen_size) {
            return @max(0.0, @min(1.0, (test_value - current_threshold) / (prev_threshold - current_threshold)));
        } else {
            return @max(0.0, @min(1.0, (current_threshold - test_value) / (current_threshold - prev_threshold)));
        }
    }
};

/// Specialized hex tile LOD calculator
pub const HexLODCalculator = struct {
    const Self = @This();

    base_calculator: LODCalculator,
    tile_size: f32,
    max_visible_distance: f32,

    pub fn init(camera_position: frustum.Vec3, camera_forward: frustum.Vec3, viewport_width: f32, viewport_height: f32, fov_y: f32, tile_size: f32) Self {
        return Self{
            .base_calculator = LODCalculator.init(camera_position, camera_forward, viewport_width, viewport_height, fov_y),
            .tile_size = tile_size,
            .max_visible_distance = tile_size * 100.0, // Adjust based on your world scale
        };
    }

    pub fn updateCamera(self: *Self, position: frustum.Vec3, forward: frustum.Vec3) void {
        self.base_calculator.updateCamera(position, forward);
    }

    /// Calculate LODs for hex tiles with specialized configuration
    pub fn calculateHexTileLODs(self: Self, coords: []const hex.HexCoord, elevations: []const f32, results: []LODResult) void {
        std.debug.assert(coords.len == elevations.len);
        std.debug.assert(coords.len == results.len);

        // Convert hex coordinates to world positions
        var positions = std.ArrayList(frustum.Vec3).init(std.heap.page_allocator);
        defer positions.deinit();

        var radii = std.ArrayList(f32).init(std.heap.page_allocator);
        defer radii.deinit();

        for (coords, elevations) |coord, elevation| {
            const pixel_pos = hex.toPixel(coord.q, coord.r, self.tile_size);

            positions.append(frustum.Vec3.init(pixel_pos.x, elevation * 0.5, // Adjust elevation scaling
                pixel_pos.y)) catch unreachable;

            radii.append(self.tile_size * 0.866025404) catch unreachable; // Hex circumradius
        }

        // Use terrain-specific LOD configuration
        const config = LODConfig.forTerrain();

        self.base_calculator.calculateLODBatch(positions.items, radii.items, config, results);
    }

    /// Get appropriate mesh complexity for LOD level
    pub fn getMeshComplexity(self: Self, level: LODLevel) struct { vertices: u32, triangles: u32 } {
        _ = self;

        return switch (level) {
            .highest => .{ .vertices = 512, .triangles = 1024 },
            .high => .{ .vertices = 256, .triangles = 512 },
            .medium => .{ .vertices = 128, .triangles = 256 },
            .low => .{ .vertices = 64, .triangles = 128 },
            .lowest => .{ .vertices = 32, .triangles = 64 },
        };
    }

    /// Get texture resolution for LOD level
    pub fn getTextureResolution(self: Self, level: LODLevel) u32 {
        _ = self;

        return switch (level) {
            .highest => 1024,
            .high => 512,
            .medium => 256,
            .low => 128,
            .lowest => 64,
        };
    }
};

/// Adaptive LOD system that adjusts based on performance
pub const AdaptiveLODSystem = struct {
    const Self = @This();

    target_frametime: f32, // Target frame time in milliseconds
    current_frametime: f32,
    performance_history: [60]f32, // 1 second of history at 60fps
    history_index: u32,
    adaptation_rate: f32,
    min_performance_scale: f32,
    max_performance_scale: f32,

    pub fn init(target_fps: f32) Self {
        return Self{
            .target_frametime = 1000.0 / target_fps,
            .current_frametime = 16.67, // 60 FPS default
            .performance_history = [_]f32{16.67} ** 60,
            .history_index = 0,
            .adaptation_rate = 0.05,
            .min_performance_scale = 0.5,
            .max_performance_scale = 2.0,
        };
    }

    pub fn updateFrametime(self: *Self, frametime_ms: f32) void {
        self.current_frametime = frametime_ms;
        self.performance_history[self.history_index] = frametime_ms;
        self.history_index = (self.history_index + 1) % 60;
    }

    pub fn getPerformanceScale(self: Self) f32 {
        // Calculate average frametime over recent history
        var sum: f32 = 0.0;
        for (self.performance_history) |frametime| {
            sum += frametime;
        }
        const avg_frametime = sum / 60.0;

        // Calculate performance ratio
        const performance_ratio = self.target_frametime / avg_frametime;

        // Apply adaptation rate for smooth changes
        const adapted_scale = 1.0 + (performance_ratio - 1.0) * self.adaptation_rate;

        return @max(self.min_performance_scale, @min(self.max_performance_scale, adapted_scale));
    }

    pub fn shouldReduceQuality(self: Self) bool {
        return self.current_frametime > self.target_frametime * 1.2; // 20% tolerance
    }

    pub fn shouldIncreaseQuality(self: Self) bool {
        return self.current_frametime < self.target_frametime * 0.8; // Running well
    }
};
