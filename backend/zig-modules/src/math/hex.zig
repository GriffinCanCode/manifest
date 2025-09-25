//! Hexagonal grid mathematics with SIMD optimizations
//!
//! Provides high-performance deterministic hexagonal grid operations
//! for grand strategy game spatial calculations.

const std = @import("std");
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

/// Get all 6 neighbors of a hex coordinate
pub fn getNeighbors(coord: HexCoord) [6]HexCoord {
    const directions = [_][2]i32{
        .{ 1, 0 }, // East
        .{ 1, -1 }, // Northeast
        .{ 0, -1 }, // Northwest
        .{ -1, 0 }, // West
        .{ -1, 1 }, // Southwest
        .{ 0, 1 }, // Southeast
    };

    var neighbors: [6]HexCoord = undefined;
    for (directions, 0..) |dir, i| {
        neighbors[i] = HexCoord.init(coord.q + dir[0], coord.r + dir[1]);
    }

    return neighbors;
}

/// Get all hex coordinates in a range from center
pub fn getHexesInRange(center: HexCoord, range: u32, allocator: std.mem.Allocator) ![]HexCoord {
    var hexes = std.ArrayList(HexCoord).init(allocator);
    defer hexes.deinit();

    const range_i = @as(i32, @intCast(range));
    var q: i32 = -range_i;

    while (q <= range_i) : (q += 1) {
        const r1 = @max(-range_i, -q - range_i);
        const r2 = @min(range_i, -q + range_i);

        var r: i32 = r1;
        while (r <= r2) : (r += 1) {
            const hex = HexCoord.init(center.q + q, center.r + r);
            try hexes.append(hex);
        }
    }

    return hexes.toOwnedSlice();
}

/// Line drawing between two hex coordinates (Bresenham-style)
pub fn drawLine(start: HexCoord, end: HexCoord, allocator: std.mem.Allocator) ![]HexCoord {
    const dist = distance(start.q, start.r, end.q, end.r);
    if (dist == 0) return try allocator.dupe(HexCoord, &[_]HexCoord{start});

    var line = std.ArrayList(HexCoord).init(allocator);
    defer line.deinit();

    var i: u32 = 0;
    while (i <= dist) : (i += 1) {
        const t = @as(f32, @floatFromInt(i)) / @as(f32, @floatFromInt(dist));

        const start_q = @as(f32, @floatFromInt(start.q));
        const start_r = @as(f32, @floatFromInt(start.r));
        const end_q = @as(f32, @floatFromInt(end.q));
        const end_r = @as(f32, @floatFromInt(end.r));

        const lerp_q = precise.detLerp(start_q, end_q, t);
        const lerp_r = precise.detLerp(start_r, end_r, t);

        const hex = roundToHex(lerp_q, lerp_r);
        try line.append(hex);
    }

    return line.toOwnedSlice();
}

/// Calculate field of view from a hex position
pub fn calculateFOV(center: HexCoord, radius: u32, is_blocked: *const fn (HexCoord) bool, allocator: std.mem.Allocator) ![]HexCoord {
    var visible = std.ArrayList(HexCoord).init(allocator);
    defer visible.deinit();

    // Add center (always visible)
    try visible.append(center);

    // Cast rays in all directions
    var angle: f32 = 0.0;
    const angle_step = precise.detDiv(2.0 * std.math.pi, 360.0); // 1-degree steps

    while (angle < 2.0 * std.math.pi) : (angle = precise.detAdd(angle, angle_step)) {
        const dx = precise.detCos(angle);
        const dy = precise.detSin(angle);

        var step: u32 = 1;
        while (step <= radius) : (step += 1) {
            const step_f = @as(f32, @floatFromInt(step));
            const target_x = precise.detAdd(@as(f32, @floatFromInt(center.q)), precise.detMul(dx, step_f));
            const target_y = precise.detAdd(@as(f32, @floatFromInt(center.r)), precise.detMul(dy, step_f));

            const target_hex = roundToHex(target_x, target_y);

            // Add to visible if not already added
            var already_added = false;
            for (visible.items) |visible_hex| {
                if (visible_hex.q == target_hex.q and visible_hex.r == target_hex.r) {
                    already_added = true;
                    break;
                }
            }

            if (!already_added) {
                try visible.append(target_hex);
            }

            // Stop if blocked
            if (is_blocked(target_hex)) {
                break;
            }
        }
    }

    return visible.toOwnedSlice();
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

    // Test pixel conversion
    const pixel = toPixel(0, 0, 10.0);
    try testing.expect(pixel.x == 0.0);
    try testing.expect(pixel.y == 0.0);

    // Test neighbors
    const neighbors = getNeighbors(HexCoord.init(0, 0));
    try testing.expect(neighbors.len == 6);
    try testing.expect(neighbors[0].q == 1 and neighbors[0].r == 0);
}
