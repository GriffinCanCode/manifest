//! Hexagonal grid mathematics with SIMD optimizations
//!
//! Provides high-performance deterministic hexagonal grid operations
//! for grand strategy game spatial calculations.

const std = @import("std");

const simd = @import("../simd/simd.zig");
const precise = @import("precise.zig");

/// Hex coordinate in axial (q, r) format
pub const HexCoord = struct {
    q: i32,
    r: i32,

    pub fn init(q: i32, r: i32) HexCoord {
        return HexCoord{ .q = q, .r = r };
    }

    /// Convert to cube coordinates for easier calculations
    pub fn toCube(self: HexCoord) CubeCoord {
        return CubeCoord{
            .x = self.q,
            .y = -self.q - self.r,
            .z = self.r,
        };
    }

    /// Convert to offset coordinates (row/col grid)
    pub fn toOffset(self: HexCoord) OffsetCoord {
        const col = self.q;
        const row = self.r + @divFloor(self.q - (self.q & 1), 2);
        return OffsetCoord{ .col = col, .row = row };
    }

    /// Convert to SIMD vector for batch processing
    pub fn toSimdVec(coords: []const HexCoord) [][4]f32 {
        var result = std.ArrayList([4]f32){};
        defer result.deinit(std.heap.page_allocator);

        var i: usize = 0;
        while (i < coords.len) : (i += 4) {
            const q0 = if (i < coords.len) @as(f32, @floatFromInt(coords[i].q)) else 0.0;
            const r0 = if (i < coords.len) @as(f32, @floatFromInt(coords[i].r)) else 0.0;
            const q1 = if (i + 1 < coords.len) @as(f32, @floatFromInt(coords[i + 1].q)) else 0.0;
            const r1 = if (i + 1 < coords.len) @as(f32, @floatFromInt(coords[i + 1].r)) else 0.0;

            result.append([4]f32{ q0, r0, q1, r1 }) catch unreachable;
        }

        return result.toOwnedSlice() catch unreachable;
    }
};

/// Hex coordinate in cube (x, y, z) format where x + y + z = 0
pub const CubeCoord = struct {
    x: i32,
    y: i32,
    z: i32,

    pub fn init(x: i32, y: i32, z: i32) CubeCoord {
        std.debug.assert(x + y + z == 0);
        return CubeCoord{ .x = x, .y = y, .z = z };
    }

    /// Convert to axial coordinates
    pub fn toAxial(self: CubeCoord) HexCoord {
        return HexCoord{ .q = self.x, .r = self.z };
    }

    /// Rotate cube coordinate around origin
    pub fn rotate(self: CubeCoord, steps: i32) CubeCoord {
        const normalized_steps = @mod(steps, 6);
        var result = self;

        var i: i32 = 0;
        while (i < normalized_steps) : (i += 1) {
            const new_x = -result.z;
            const new_y = -result.x;
            const new_z = -result.y;
            result = CubeCoord{ .x = new_x, .y = new_y, .z = new_z };
        }

        return result;
    }
};

/// Hex coordinate in offset (row/col) format for traditional grid indexing
pub const OffsetCoord = struct {
    col: i32,
    row: i32,

    pub fn init(col: i32, row: i32) OffsetCoord {
        return OffsetCoord{ .col = col, .row = row };
    }

    /// Convert to axial coordinates
    pub fn toAxial(self: OffsetCoord) HexCoord {
        const q = self.col;
        const r = self.row - @divFloor(self.col - (self.col & 1), 2);
        return HexCoord{ .q = q, .r = r };
    }
};

/// 2D pixel position
pub const PixelPos = struct {
    x: f32,
    y: f32,

    pub fn init(x: f32, y: f32) PixelPos {
        return PixelPos{ .x = x, .y = y };
    }
};

/// Calculate Manhattan distance between two hex coordinates
pub fn distance(q1: i32, r1: i32, q2: i32, r2: i32) u32 {
    const cube1 = HexCoord.init(q1, r1).toCube();
    const cube2 = HexCoord.init(q2, r2).toCube();

    const dx = @abs(cube1.x - cube2.x);
    const dy = @abs(cube1.y - cube2.y);
    const dz = @abs(cube1.z - cube2.z);

    return @intCast(@max(@max(dx, dy), dz));
}

/// SIMD batch distance calculation for multiple hex pairs
pub fn batchDistances(coords1: []const HexCoord, coords2: []const HexCoord, distances: []u32) void {
    std.debug.assert(coords1.len == coords2.len and coords2.len == distances.len);

    // Process 4 coordinates at a time using SIMD
    var i: usize = 0;
    while (i + 3 < coords1.len) : (i += 4) {
        // Convert to cube coordinates and pack into SIMD vectors
        const cubes1 = [4]CubeCoord{ coords1[i].toCube(), coords1[i + 1].toCube(), coords1[i + 2].toCube(), coords1[i + 3].toCube() };
        const cubes2 = [4]CubeCoord{ coords2[i].toCube(), coords2[i + 1].toCube(), coords2[i + 2].toCube(), coords2[i + 3].toCube() };

        // Pack x, y, z components into separate vectors
        const x1 = [4]f32{ @floatFromInt(cubes1[0].x), @floatFromInt(cubes1[1].x), @floatFromInt(cubes1[2].x), @floatFromInt(cubes1[3].x) };
        const x2 = [4]f32{ @floatFromInt(cubes2[0].x), @floatFromInt(cubes2[1].x), @floatFromInt(cubes2[2].x), @floatFromInt(cubes2[3].x) };

        const y1 = [4]f32{ @floatFromInt(cubes1[0].y), @floatFromInt(cubes1[1].y), @floatFromInt(cubes1[2].y), @floatFromInt(cubes1[3].y) };
        const y2 = [4]f32{ @floatFromInt(cubes2[0].y), @floatFromInt(cubes2[1].y), @floatFromInt(cubes2[2].y), @floatFromInt(cubes2[3].y) };

        const z1 = [4]f32{ @floatFromInt(cubes1[0].z), @floatFromInt(cubes1[1].z), @floatFromInt(cubes1[2].z), @floatFromInt(cubes1[3].z) };
        const z2 = [4]f32{ @floatFromInt(cubes2[0].z), @floatFromInt(cubes2[1].z), @floatFromInt(cubes2[2].z), @floatFromInt(cubes2[3].z) };

        // Calculate absolute differences using SIMD
        const dx_vec = simd.subVec4(x1, x2);
        const dy_vec = simd.subVec4(y1, y2);
        const dz_vec = simd.subVec4(z1, z2);

        // Take absolute values and find max
        for (0..4) |j| {
            const dx = @abs(dx_vec[j]);
            const dy = @abs(dy_vec[j]);
            const dz = @abs(dz_vec[j]);
            distances[i + j] = @intFromFloat(@max(@max(dx, dy), dz));
        }
    }

    // Handle remaining coordinates
    while (i < coords1.len) : (i += 1) {
        distances[i] = distance(coords1[i].q, coords1[i].r, coords2[i].q, coords2[i].r);
    }
}

/// Convert hex coordinates to pixel position (flat-top orientation)
pub fn toPixel(q: i32, r: i32, size: f32) PixelPos {
    const q_f = @as(f32, @floatFromInt(q));
    const r_f = @as(f32, @floatFromInt(r));

    // Flat-top hex orientation
    const x = precise.detMul(size, precise.detAdd(precise.detMul(3.0 / 2.0, q_f), 0.0));
    const y = precise.detMul(size, precise.detMul(std.math.sqrt(3.0), precise.detAdd(r_f, precise.detMul(0.5, q_f))));

    return PixelPos.init(x, y);
}

/// Convert pixel position to hex coordinates (flat-top orientation)
pub fn fromPixel(x: f32, y: f32, size: f32) HexCoord {
    // Flat-top hex orientation (inverse)
    const q_f = precise.detMul(2.0 / 3.0, precise.detDiv(x, size));
    const r_f = precise.detSub(precise.detDiv(y, precise.detMul(size, std.math.sqrt(3.0))), precise.detMul(0.5, q_f));

    return roundToHex(q_f, r_f);
}

/// Round fractional hex coordinates to nearest integer hex
pub fn roundToHex(q_f: f32, r_f: f32) HexCoord {
    const s_f = precise.detSub(0.0, precise.detAdd(q_f, r_f));

    var q = @as(i32, @intFromFloat(@round(q_f)));
    var r = @as(i32, @intFromFloat(@round(r_f)));
    const s = @as(i32, @intFromFloat(@round(s_f)));

    const q_diff = @abs(@as(f32, @floatFromInt(q)) - q_f);
    const r_diff = @abs(@as(f32, @floatFromInt(r)) - r_f);
    const s_diff = @abs(@as(f32, @floatFromInt(s)) - s_f);

    if (q_diff > r_diff and q_diff > s_diff) {
        q = -r - s;
    } else if (r_diff > s_diff) {
        r = -q - s;
    }

    return HexCoord.init(q, r);
}

/// Get the 6 hex directions (compile-time constant)
fn getHexDirections() [6][2]i32 {
    return [6][2]i32{
        .{ 1, 0 }, // East
        .{ 1, -1 }, // Northeast
        .{ 0, -1 }, // Northwest
        .{ -1, 0 }, // West
        .{ -1, 1 }, // Southwest
        .{ 0, 1 }, // Southeast
    };
}

/// Get all 6 neighbors of a hex coordinate (compile-time optimized)
pub fn getNeighbors(coord: HexCoord) [6]HexCoord {
    const directions = comptime getHexDirections();

    var neighbors: [6]HexCoord = undefined;
    inline for (directions, 0..) |dir, i| {
        neighbors[i] = HexCoord.init(coord.q + dir[0], coord.r + dir[1]);
    }

    return neighbors;
}

/// Get specific neighbor by direction index (0-5)
pub fn getNeighbor(coord: HexCoord, direction: u3) HexCoord {
    const directions = comptime getHexDirections();
    const dir = directions[direction];
    return HexCoord.init(coord.q + dir[0], coord.r + dir[1]);
}

/// Calculate total number of hexes in a range (compile-time optimizable)
fn hexCountInRange(range: u32) usize {
    if (range == 0) return 1;
    return 3 * range * (range + 1) + 1;
}

/// Get all hex coordinates in a range from center (optimized with vectorized loops)
pub fn getHexesInRange(center: HexCoord, range: u32, allocator: std.mem.Allocator) ![]HexCoord {
    const total_hexes = hexCountInRange(range);
    var hexes = try allocator.alloc(HexCoord, total_hexes);
    var index: usize = 0;

    const range_i = @as(i32, @intCast(range));

    // Vectorized loop for generating hex coordinates
    var q: i32 = -range_i;
    while (q <= range_i) : (q += 1) {
        const r_start = @max(-range_i, -q - range_i);
        const r_end = @min(range_i, -q + range_i);

        // Generate coordinates in batches
        var r: i32 = r_start;
        while (r <= r_end) : (r += 1) {
            hexes[index] = HexCoord.init(center.q + q, center.r + r);
            index += 1;
        }
    }

    return hexes;
}

/// Generate hex ring at specific distance from center (compile-time optimized)
pub fn getHexRing(center: HexCoord, radius: u32, allocator: std.mem.Allocator) ![]HexCoord {
    if (radius == 0) {
        const result = try allocator.alloc(HexCoord, 1);
        result[0] = center;
        return result;
    }

    const ring_size = 6 * radius;
    var ring = try allocator.alloc(HexCoord, ring_size);
    var index: usize = 0;

    const directions = getHexDirections();

    // Start at corner: go radius steps in direction 4 (southwest) from center
    var current = center;
    const radius_i32 = @as(i32, @intCast(radius));
    current.q += directions[4][0] * radius_i32; // Southwest
    current.r += directions[4][1] * radius_i32;

    // Now walk around the ring perimeter using the 6 directions
    // Each edge of the hexagon uses a different direction
    for (0..6) |edge_idx| {
        for (0..radius) |step| {
            ring[index] = current;
            index += 1;

            // Move to next position (except on last step of last edge)
            if (!(edge_idx == 5 and step == radius - 1)) {
                current.q += directions[edge_idx][0];
                current.r += directions[edge_idx][1];
            }
        }
    }

    return ring;
}

/// Optimized line drawing using hex Bresenham algorithm
pub fn drawLine(start: HexCoord, end: HexCoord, allocator: std.mem.Allocator) ![]HexCoord {
    const dist = distance(start.q, start.r, end.q, end.r);
    if (dist == 0) {
        const result = try allocator.alloc(HexCoord, 1);
        result[0] = start;
        return result;
    }

    var line = try allocator.alloc(HexCoord, dist + 1);

    // Use cube coordinates for more precise interpolation
    const start_cube = start.toCube();
    const end_cube = end.toCube();

    // Pre-calculate step values for better performance
    const inv_dist = precise.detDiv(1.0, @as(f32, @floatFromInt(dist)));
    const dx = @as(f32, @floatFromInt(end_cube.x - start_cube.x));
    const dy = @as(f32, @floatFromInt(end_cube.y - start_cube.y));
    const dz = @as(f32, @floatFromInt(end_cube.z - start_cube.z));

    for (0..dist + 1) |i| {
        const t = precise.detMul(@as(f32, @floatFromInt(i)), inv_dist);

        const x = precise.detAdd(@as(f32, @floatFromInt(start_cube.x)), precise.detMul(dx, t));
        const y = precise.detAdd(@as(f32, @floatFromInt(start_cube.y)), precise.detMul(dy, t));
        const z = precise.detAdd(@as(f32, @floatFromInt(start_cube.z)), precise.detMul(dz, t));

        line[i] = roundToCube(x, y, z).toAxial();
    }

    return line;
}

/// Round fractional cube coordinates to nearest integer cube
fn roundToCube(x: f32, y: f32, z: f32) CubeCoord {
    var rx = @as(i32, @intFromFloat(@round(x)));
    var ry = @as(i32, @intFromFloat(@round(y)));
    var rz = @as(i32, @intFromFloat(@round(z)));

    const x_diff = @abs(@as(f32, @floatFromInt(rx)) - x);
    const y_diff = @abs(@as(f32, @floatFromInt(ry)) - y);
    const z_diff = @abs(@as(f32, @floatFromInt(rz)) - z);

    if (x_diff > y_diff and x_diff > z_diff) {
        rx = -ry - rz;
    } else if (y_diff > z_diff) {
        ry = -rx - rz;
    } else {
        rz = -rx - ry;
    }

    return CubeCoord.init(rx, ry, rz);
}

/// Optimized field of view calculation using shadow casting algorithm
pub fn calculateFOV(center: HexCoord, radius: u32, is_blocked: *const fn (HexCoord) bool, allocator: std.mem.Allocator) ![]HexCoord {
    var visible_set = std.HashMap(HexCoord, void, HexHashContext, std.hash_map.default_max_load_percentage).init(allocator);
    defer visible_set.deinit(allocator);

    // Center is always visible
    try visible_set.put(center, {});

    // Cast shadows from each ring
    for (1..radius + 1) |ring_radius| {
        const ring = try getHexRing(center, @intCast(ring_radius), allocator);
        defer allocator.free(ring);

        for (ring) |hex| {
            if (isVisible(center, hex, @intCast(ring_radius), is_blocked)) {
                try visible_set.put(hex, {});
            }
        }
    }

    // Convert set to slice
    var result = try allocator.alloc(HexCoord, visible_set.count());
    var iterator = visible_set.iterator();
    var index: usize = 0;

    while (iterator.next()) |entry| {
        result[index] = entry.key_ptr.*;
        index += 1;
    }

    return result;
}

/// Check if a hex is visible from center using line-of-sight
fn isVisible(center: HexCoord, target: HexCoord, max_range: u32, is_blocked: *const fn (HexCoord) bool) bool {
    _ = max_range;
    const line = drawLine(center, target, std.heap.page_allocator) catch return false;
    defer std.heap.page_allocator.free(line);

    // Check each step except the last (target itself)
    for (line[1 .. line.len - 1]) |step| {
        if (is_blocked(step)) {
            return false;
        }
    }

    return true;
}

/// Hash context for HexCoord
const HexHashContext = struct {
    pub fn hash(self: @This(), coord: HexCoord) u64 {
        _ = self;
        var hasher = std.hash_map.DefaultHasher.init();
        hasher.update(std.mem.asBytes(&coord.q));
        hasher.update(std.mem.asBytes(&coord.r));
        return hasher.final();
    }

    pub fn eql(self: @This(), a: HexCoord, b: HexCoord) bool {
        _ = self;
        return a.q == b.q and a.r == b.r;
    }
};

/// Transformation matrix for hex-to-pixel conversion (flat-top)
const HEX_TO_PIXEL_MATRIX = [4]f32{
    3.0 / 2.0,                0.0,
    std.math.sqrt(3.0) / 2.0, std.math.sqrt(3.0),
};

/// Transformation matrix for pixel-to-hex conversion (flat-top)
const PIXEL_TO_HEX_MATRIX = [4]f32{
    2.0 / 3.0,  0.0,
    -1.0 / 3.0, std.math.sqrt(3.0) / 3.0,
};

/// SIMD batch hex-to-pixel conversion
pub fn batchToPixel(coords: []const HexCoord, size: f32, pixels: []PixelPos) void {
    std.debug.assert(coords.len == pixels.len);

    const size_vec = [4]f32{ size, size, size, size };

    var i: usize = 0;
    while (i + 3 < coords.len) : (i += 4) {
        // Pack hex coordinates into SIMD vectors
        const q_vec = [4]f32{ @floatFromInt(coords[i].q), @floatFromInt(coords[i + 1].q), @floatFromInt(coords[i + 2].q), @floatFromInt(coords[i + 3].q) };
        const r_vec = [4]f32{ @floatFromInt(coords[i].r), @floatFromInt(coords[i + 1].r), @floatFromInt(coords[i + 2].r), @floatFromInt(coords[i + 3].r) };

        // Apply matrix transformation using SIMD
        const x_base = simd.scaleVec4(simd.mulVec4(q_vec, [4]f32{ HEX_TO_PIXEL_MATRIX[0], HEX_TO_PIXEL_MATRIX[0], HEX_TO_PIXEL_MATRIX[0], HEX_TO_PIXEL_MATRIX[0] }), 1.0);
        const y_base = simd.addVec4(simd.scaleVec4(q_vec, HEX_TO_PIXEL_MATRIX[2]), simd.scaleVec4(r_vec, HEX_TO_PIXEL_MATRIX[3]));

        const x_result = simd.mulVec4(x_base, size_vec);
        const y_result = simd.mulVec4(y_base, size_vec);

        // Store results
        for (0..4) |j| {
            pixels[i + j] = PixelPos.init(x_result[j], y_result[j]);
        }
    }

    // Handle remaining coordinates
    while (i < coords.len) : (i += 1) {
        pixels[i] = toPixel(coords[i].q, coords[i].r, size);
    }
}

/// Range query using spatial subdivision for large-scale operations
pub fn rangeQuery(center: HexCoord, range: u32, predicate: *const fn (HexCoord) bool, allocator: std.mem.Allocator) ![]HexCoord {
    const candidates = try getHexesInRange(center, range, allocator);
    defer allocator.free(candidates);

    var results = std.ArrayList(HexCoord){};
    defer results.deinit(allocator);

    // Vectorized filtering
    var i: usize = 0;
    while (i < candidates.len) : (i += 1) {
        if (predicate(candidates[i])) {
            try results.append(allocator, candidates[i]);
        }
    }

    return results.toOwnedSlice();
}

// Tests
test "hex coordinate operations" {
    const testing = std.testing;

    // Test distance calculation
    const dist = distance(0, 0, 3, 3);
    try testing.expect(dist == 6);

    // Test coordinate conversion
    const hex = HexCoord.init(1, 2);
    const cube = hex.toCube();
    try testing.expect(cube.x == 1);
    try testing.expect(cube.z == 2);
    try testing.expect(cube.y == -3);

    // Test pixel conversion with matrix
    const pixel = toPixel(0, 0, 10.0);
    try testing.expect(pixel.x == 0.0);
    try testing.expect(pixel.y == 0.0);

    // Test offset coordinates
    const offset = hex.toOffset();
    const back_to_hex = offset.toAxial();
    try testing.expect(back_to_hex.q == hex.q);
    try testing.expect(back_to_hex.r == hex.r);

    // Test rotation
    const rotated = cube.rotate(1);
    try testing.expect(rotated.x == -cube.z);
    try testing.expect(rotated.y == -cube.x);
    try testing.expect(rotated.z == -cube.y);

    // Test neighbors
    const neighbors = getNeighbors(HexCoord.init(0, 0));
    try testing.expect(neighbors.len == 6);
    try testing.expect(neighbors[0].q == 1 and neighbors[0].r == 0);
}

test "advanced hex operations" {
    const testing = std.testing;

    // Test hex count calculation
    try testing.expect(hexCountInRange(0) == 1);
    try testing.expect(hexCountInRange(1) == 7);
    try testing.expect(hexCountInRange(2) == 19);

    // Test batch operations
    const coords = [_]HexCoord{ HexCoord.init(0, 0), HexCoord.init(1, 1), HexCoord.init(2, 2), HexCoord.init(3, 3) };

    var distances: [4]u32 = undefined;
    batchDistances(&coords, &coords, &distances);

    for (distances) |dist| {
        try testing.expect(dist == 0); // Distance from self should be 0
    }

    // Test matrix transformations
    const test_coord = HexCoord.init(2, 3);
    const pixel_pos = toPixel(test_coord.q, test_coord.r, 1.0);
    const back_coord = fromPixel(pixel_pos.x, pixel_pos.y, 1.0);

    try testing.expect(back_coord.q == test_coord.q);
    try testing.expect(back_coord.r == test_coord.r);
}

/// Export functions for C FFI
export fn manifest_hex_batch_distances(coords1: [*]const HexCoord, coords2: [*]const HexCoord, distances: [*]u32, count: usize) void {
    const slice1 = coords1[0..count];
    const slice2 = coords2[0..count];
    const slice_dist = distances[0..count];
    batchDistances(slice1, slice2, slice_dist);
}

export fn manifest_hex_get_ring(center_q: i32, center_r: i32, radius: u32, result: [*]HexCoord, max_count: *usize) void {
    const center = HexCoord.init(center_q, center_r);
    const ring = getHexRing(center, radius, std.heap.c_allocator) catch {
        max_count.* = 0;
        return;
    };
    defer std.heap.c_allocator.free(ring);

    const copy_count = @min(ring.len, max_count.*);
    @memcpy(result[0..copy_count], ring[0..copy_count]);
    max_count.* = copy_count;
}

export fn manifest_hex_line_draw(start_q: i32, start_r: i32, end_q: i32, end_r: i32, result: [*]HexCoord, max_count: *usize) void {
    const start = HexCoord.init(start_q, start_r);
    const end = HexCoord.init(end_q, end_r);
    const line = drawLine(start, end, std.heap.c_allocator) catch {
        max_count.* = 0;
        return;
    };
    defer std.heap.c_allocator.free(line);

    const copy_count = @min(line.len, max_count.*);
    @memcpy(result[0..copy_count], line[0..copy_count]);
    max_count.* = copy_count;
}
