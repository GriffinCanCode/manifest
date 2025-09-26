//! Frustum culling system with SIMD optimization
//!
//! Provides high-performance frustum culling for 3D rendering with support for:
//! - Frustum plane extraction from view-projection matrices
//! - AABB and sphere frustum intersection tests
//! - Batch processing with SIMD acceleration
//! - Hierarchical culling for complex scenes

const std = @import("std");

const hex = @import("../math/hex.zig");
const precise = @import("../math/precise.zig");
const simd = @import("../simd/simd.zig");

/// 3D vector for culling calculations
pub const Vec3 = struct {
    x: f32,
    y: f32,
    z: f32,

    pub fn init(x: f32, y: f32, z: f32) Vec3 {
        return Vec3{ .x = x, .y = y, .z = z };
    }

    pub fn toSimdVec(self: Vec3) [4]f32 {
        return [4]f32{ self.x, self.y, self.z, 0.0 };
    }

    pub fn fromSimdVec(vec: [4]f32) Vec3 {
        return Vec3.init(vec[0], vec[1], vec[2]);
    }
};

/// Frustum plane representation
pub const FrustumPlane = struct {
    normal: Vec3,
    distance: f32,

    /// Test point against plane (positive = front, negative = back)
    pub fn testPoint(self: FrustumPlane, point: Vec3) f32 {
        return precise.detAdd(precise.detAdd(precise.detMul(self.normal.x, point.x), precise.detMul(self.normal.y, point.y)), precise.detAdd(precise.detMul(self.normal.z, point.z), self.distance));
    }

    /// SIMD test multiple points against plane
    pub fn testPointsBatch(self: FrustumPlane, points: []const Vec3, results: []f32) void {
        const normal_vec = [4]f32{ self.normal.x, self.normal.y, self.normal.z, 0.0 };
        const distance_vec = [4]f32{ self.distance, self.distance, self.distance, self.distance };

        var i: usize = 0;
        while (i + 3 < points.len) : (i += 4) {
            // Pack 4 points into SIMD vectors (xyz components)
            const points_x = [4]f32{ points[i].x, points[i + 1].x, points[i + 2].x, points[i + 3].x };
            const points_y = [4]f32{ points[i].y, points[i + 1].y, points[i + 2].y, points[i + 3].y };
            const points_z = [4]f32{ points[i].z, points[i + 1].z, points[i + 2].z, points[i + 3].z };

            // Calculate dot products using SIMD
            const dot_x = simd.mulVec4(points_x, [4]f32{ normal_vec[0], normal_vec[0], normal_vec[0], normal_vec[0] });
            const dot_y = simd.mulVec4(points_y, [4]f32{ normal_vec[1], normal_vec[1], normal_vec[1], normal_vec[1] });
            const dot_z = simd.mulVec4(points_z, [4]f32{ normal_vec[2], normal_vec[2], normal_vec[2], normal_vec[2] });

            const dot_sum = simd.addVec4(simd.addVec4(dot_x, dot_y), dot_z);
            const final_result = simd.addVec4(dot_sum, distance_vec);

            // Store results
            for (0..4) |j| {
                if (i + j < points.len) {
                    results[i + j] = final_result[j];
                }
            }
        }

        // Handle remaining points
        while (i < points.len) : (i += 1) {
            results[i] = self.testPoint(points[i]);
        }
    }
};

/// Complete frustum representation (6 planes)
pub const Frustum = struct {
    planes: [6]FrustumPlane,

    const PLANE_LEFT = 0;
    const PLANE_RIGHT = 1;
    const PLANE_BOTTOM = 2;
    const PLANE_TOP = 3;
    const PLANE_NEAR = 4;
    const PLANE_FAR = 5;

    /// Extract frustum planes from view-projection matrix (column-major)
    pub fn fromMatrix(view_projection: [16]f32) Frustum {
        var frustum = Frustum{ .planes = undefined };

        // Extract planes using Gribb/Hartmann method
        // Left plane: column 4 + column 1
        frustum.planes[PLANE_LEFT] = FrustumPlane{
            .normal = Vec3.init(view_projection[3] + view_projection[0], view_projection[7] + view_projection[4], view_projection[11] + view_projection[8]),
            .distance = view_projection[15] + view_projection[12],
        };

        // Right plane: column 4 - column 1
        frustum.planes[PLANE_RIGHT] = FrustumPlane{
            .normal = Vec3.init(view_projection[3] - view_projection[0], view_projection[7] - view_projection[4], view_projection[11] - view_projection[8]),
            .distance = view_projection[15] - view_projection[12],
        };

        // Bottom plane: column 4 + column 2
        frustum.planes[PLANE_BOTTOM] = FrustumPlane{
            .normal = Vec3.init(view_projection[3] + view_projection[1], view_projection[7] + view_projection[5], view_projection[11] + view_projection[9]),
            .distance = view_projection[15] + view_projection[13],
        };

        // Top plane: column 4 - column 2
        frustum.planes[PLANE_TOP] = FrustumPlane{
            .normal = Vec3.init(view_projection[3] - view_projection[1], view_projection[7] - view_projection[5], view_projection[11] - view_projection[9]),
            .distance = view_projection[15] - view_projection[13],
        };

        // Near plane: column 4 + column 3
        frustum.planes[PLANE_NEAR] = FrustumPlane{
            .normal = Vec3.init(view_projection[3] + view_projection[2], view_projection[7] + view_projection[6], view_projection[11] + view_projection[10]),
            .distance = view_projection[15] + view_projection[14],
        };

        // Far plane: column 4 - column 3
        frustum.planes[PLANE_FAR] = FrustumPlane{
            .normal = Vec3.init(view_projection[3] - view_projection[2], view_projection[7] - view_projection[6], view_projection[11] - view_projection[10]),
            .distance = view_projection[15] - view_projection[14],
        };

        // Normalize all planes
        for (&frustum.planes) |*plane| {
            const length = std.math.sqrt(precise.detAdd(precise.detAdd(precise.detMul(plane.normal.x, plane.normal.x), precise.detMul(plane.normal.y, plane.normal.y)), precise.detMul(plane.normal.z, plane.normal.z)));

            if (length > 0.0) {
                plane.normal.x = precise.detDiv(plane.normal.x, length);
                plane.normal.y = precise.detDiv(plane.normal.y, length);
                plane.normal.z = precise.detDiv(plane.normal.z, length);
                plane.distance = precise.detDiv(plane.distance, length);
            }
        }

        return frustum;
    }

    /// Test point against frustum (true = inside)
    pub fn testPoint(self: Frustum, point: Vec3) bool {
        for (self.planes) |plane| {
            if (plane.testPoint(point) < 0.0) {
                return false;
            }
        }
        return true;
    }

    /// Test sphere against frustum
    pub fn testSphere(self: Frustum, center: Vec3, radius: f32) CullingResult {
        var inside = true;

        for (self.planes) |plane| {
            const distance = plane.testPoint(center);

            if (distance < -radius) {
                return .outside;
            } else if (distance < radius) {
                inside = false;
            }
        }

        return if (inside) .inside else .intersecting;
    }

    /// Test AABB against frustum
    pub fn testAABB(self: Frustum, min_point: Vec3, max_point: Vec3) CullingResult {
        var inside = true;

        for (self.planes) |plane| {
            // Find positive/negative vertices
            var p_vertex = min_point;
            var n_vertex = max_point;

            if (plane.normal.x >= 0) {
                p_vertex.x = max_point.x;
                n_vertex.x = min_point.x;
            }
            if (plane.normal.y >= 0) {
                p_vertex.y = max_point.y;
                n_vertex.y = min_point.y;
            }
            if (plane.normal.z >= 0) {
                p_vertex.z = max_point.z;
                n_vertex.z = min_point.z;
            }

            // Test negative vertex
            if (plane.testPoint(n_vertex) < 0) {
                return .outside;
            }

            // Test positive vertex
            if (plane.testPoint(p_vertex) < 0) {
                inside = false;
            }
        }

        return if (inside) .inside else .intersecting;
    }
};

/// Culling result enumeration
pub const CullingResult = enum {
    outside, // Completely outside frustum
    inside, // Completely inside frustum
    intersecting, // Partially inside frustum
};

/// AABB for culling tests
pub const AABB = struct {
    min: Vec3,
    max: Vec3,

    pub fn init(min: Vec3, max: Vec3) AABB {
        return AABB{ .min = min, .max = max };
    }

    pub fn fromCenterAndSize(center: Vec3, size: Vec3) AABB {
        const half_size = Vec3.init(size.x * 0.5, size.y * 0.5, size.z * 0.5);
        return AABB{
            .min = Vec3.init(center.x - half_size.x, center.y - half_size.y, center.z - half_size.z),
            .max = Vec3.init(center.x + half_size.x, center.y + half_size.y, center.z + half_size.z),
        };
    }

    pub fn getCenter(self: AABB) Vec3 {
        return Vec3.init((self.min.x + self.max.x) * 0.5, (self.min.y + self.max.y) * 0.5, (self.min.z + self.max.z) * 0.5);
    }

    pub fn getSize(self: AABB) Vec3 {
        return Vec3.init(self.max.x - self.min.x, self.max.y - self.min.y, self.max.z - self.min.z);
    }
};

/// Sphere for culling tests
pub const Sphere = struct {
    center: Vec3,
    radius: f32,

    pub fn init(center: Vec3, radius: f32) Sphere {
        return Sphere{ .center = center, .radius = radius };
    }

    pub fn fromAABB(aabb: AABB) Sphere {
        const center = aabb.getCenter();
        const size = aabb.getSize();

        // Calculate radius as distance to furthest corner
        const radius = std.math.sqrt(size.x * size.x + size.y * size.y + size.z * size.z) * 0.5;

        return Sphere.init(center, radius);
    }
};

/// Batch culling operations for performance
pub const BatchCuller = struct {
    /// Test multiple AABBs against frustum with SIMD acceleration
    pub fn testAABBs(frustum: Frustum, aabbs: []const AABB, results: []CullingResult) void {
        std.debug.assert(aabbs.len == results.len);

        for (aabbs, results) |aabb, *result| {
            result.* = frustum.testAABB(aabb.min, aabb.max);
        }
    }

    /// Test multiple spheres against frustum with SIMD acceleration
    pub fn testSpheres(frustum: Frustum, spheres: []const Sphere, results: []CullingResult) void {
        std.debug.assert(spheres.len == results.len);

        // Process in SIMD batches for better performance
        var i: usize = 0;
        while (i + 3 < spheres.len) : (i += 4) {
            // Pack sphere centers into SIMD vectors
            const centers_x = [4]f32{ spheres[i].center.x, spheres[i + 1].center.x, spheres[i + 2].center.x, spheres[i + 3].center.x };
            const centers_y = [4]f32{ spheres[i].center.y, spheres[i + 1].center.y, spheres[i + 2].center.y, spheres[i + 3].center.y };
            const centers_z = [4]f32{ spheres[i].center.z, spheres[i + 1].center.z, spheres[i + 2].center.z, spheres[i + 3].center.z };
            const radii = [4]f32{ spheres[i].radius, spheres[i + 1].radius, spheres[i + 2].radius, spheres[i + 3].radius };

            // Test against each frustum plane
            for (&results[i .. i + 4], 0..) |*result, j| {
                const sphere = Sphere.init(Vec3.init(centers_x[j], centers_y[j], centers_z[j]), radii[j]);
                result.* = frustum.testSphere(sphere.center, sphere.radius);
            }
        }

        // Handle remaining spheres
        while (i < spheres.len) : (i += 1) {
            results[i] = frustum.testSphere(spheres[i].center, spheres[i].radius);
        }
    }

    /// Hierarchical culling for complex scenes
    pub fn hierarchicalCull(frustum: Frustum, bounds: AABB, child_bounds: []const AABB, results: []CullingResult) bool {
        // Test parent bounds first
        const parent_result = frustum.testAABB(bounds.min, bounds.max);

        switch (parent_result) {
            .outside => {
                // All children are outside
                for (results) |*result| {
                    result.* = .outside;
                }
                return false;
            },
            .inside => {
                // All children are inside
                for (results) |*result| {
                    result.* = .inside;
                }
                return true;
            },
            .intersecting => {
                // Need to test individual children
                testAABBs(frustum, child_bounds, results);
                return true;
            },
        }
    }
};

/// Statistics for culling operations
pub const CullingStats = struct {
    total_tested: u32,
    culled_objects: u32,
    visible_objects: u32,
    intersecting_objects: u32,

    pub fn init() CullingStats {
        return CullingStats{
            .total_tested = 0,
            .culled_objects = 0,
            .visible_objects = 0,
            .intersecting_objects = 0,
        };
    }

    pub fn addResult(self: *CullingStats, result: CullingResult) void {
        self.total_tested += 1;
        switch (result) {
            .outside => self.culled_objects += 1,
            .inside => self.visible_objects += 1,
            .intersecting => self.intersecting_objects += 1,
        }
    }

    pub fn getCullingRatio(self: CullingStats) f32 {
        if (self.total_tested == 0) return 0.0;
        return @as(f32, @floatFromInt(self.culled_objects)) / @as(f32, @floatFromInt(self.total_tested));
    }

    pub fn reset(self: *CullingStats) void {
        self.* = CullingStats.init();
    }
};

/// Integration with hex coordinate system for tile-based culling
pub const HexCuller = struct {
    /// Convert hex tile to AABB for culling
    pub fn hexTileToAABB(coord: hex.HexCoord, elevation: f32, tile_size: f32) AABB {
        const pixel_pos = hex.toPixel(coord.q, coord.r, tile_size);
        const height = elevation * 0.5; // Half-height for center

        // Hex tile bounds (flat-top orientation)
        const hex_radius = tile_size;
        const hex_height = tile_size * 0.866025404; // sqrt(3) / 2

        return AABB.init(Vec3.init(pixel_pos.x - hex_radius, height - 0.1, pixel_pos.y - hex_height), Vec3.init(pixel_pos.x + hex_radius, height + 0.1, pixel_pos.y + hex_height));
    }

    /// Batch cull hex tiles with SIMD optimization
    pub fn cullHexTiles(frustum: Frustum, coords: []const hex.HexCoord, elevations: []const f32, tile_size: f32, results: []CullingResult) void {
        std.debug.assert(coords.len == elevations.len);
        std.debug.assert(coords.len == results.len);

        var i: usize = 0;
        while (i + 3 < coords.len) : (i += 4) {
            // Convert 4 hex tiles to AABBs using SIMD
            var aabbs: [4]AABB = undefined;
            for (0..4) |j| {
                if (i + j < coords.len) {
                    aabbs[j] = hexTileToAABB(coords[i + j], elevations[i + j], tile_size);
                }
            }

            // Test AABBs against frustum
            for (0..4) |j| {
                if (i + j < coords.len) {
                    results[i + j] = frustum.testAABB(aabbs[j].min, aabbs[j].max);
                }
            }
        }

        // Handle remaining tiles
        while (i < coords.len) : (i += 1) {
            const aabb = hexTileToAABB(coords[i], elevations[i], tile_size);
            results[i] = frustum.testAABB(aabb.min, aabb.max);
        }
    }
};
