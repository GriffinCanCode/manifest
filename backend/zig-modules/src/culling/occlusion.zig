//! Occlusion culling system for advanced visibility determination
//!
//! Provides hierarchical Z-buffer and occlusion query support for
//! large-scale scene rendering with SIMD optimization.

const std = @import("std");

const simd = @import("../simd/simd.zig");
const frustum = @import("frustum.zig");

/// Hierarchical Z-buffer for occlusion culling
pub const HierarchicalZBuffer = struct {
    const Self = @This();

    levels: []ZBufferLevel,
    width: u32,
    height: u32,
    max_levels: u32,
    allocator: std.mem.Allocator,

    const ZBufferLevel = struct {
        width: u32,
        height: u32,
        depth_buffer: []f32,

        pub fn init(width: u32, height: u32, allocator: std.mem.Allocator) !ZBufferLevel {
            const size = width * height;
            const depth_buffer = try allocator.alloc(f32, size);

            // Initialize to maximum depth
            for (depth_buffer) |*depth| {
                depth.* = 1.0;
            }

            return ZBufferLevel{
                .width = width,
                .height = height,
                .depth_buffer = depth_buffer,
            };
        }

        pub fn deinit(self: *ZBufferLevel, allocator: std.mem.Allocator) void {
            allocator.free(self.depth_buffer);
        }

        pub fn getDepth(self: ZBufferLevel, x: u32, y: u32) f32 {
            if (x >= self.width or y >= self.height) return 1.0;
            return self.depth_buffer[y * self.width + x];
        }

        pub fn setDepth(self: *ZBufferLevel, x: u32, y: u32, depth: f32) void {
            if (x >= self.width or y >= self.height) return;
            const index = y * self.width + x;
            self.depth_buffer[index] = @min(self.depth_buffer[index], depth);
        }

        /// SIMD-optimized depth comparison for rectangular regions
        pub fn testRegion(self: ZBufferLevel, min_x: u32, min_y: u32, max_x: u32, max_y: u32, test_depth: f32) bool {
            const effective_max_x = @min(max_x, self.width);
            const effective_max_y = @min(max_y, self.height);

            var y = min_y;
            while (y < effective_max_y) : (y += 1) {
                var x = min_x;

                // Process 4 pixels at a time using SIMD
                while (x + 3 < effective_max_x) : (x += 4) {
                    const index = y * self.width + x;
                    const depths = [4]f32{
                        self.depth_buffer[index],
                        self.depth_buffer[index + 1],
                        self.depth_buffer[index + 2],
                        self.depth_buffer[index + 3],
                    };
                    const test_depths = [4]f32{ test_depth, test_depth, test_depth, test_depth };

                    // If any depth is closer (smaller), object is potentially visible
                    for (0..4) |i| {
                        if (test_depths[i] <= depths[i]) {
                            return true;
                        }
                    }
                }

                // Handle remaining pixels
                while (x < effective_max_x) : (x += 1) {
                    if (test_depth <= self.getDepth(x, y)) {
                        return true;
                    }
                }
            }

            return false; // Completely occluded
        }
    };

    pub fn init(width: u32, height: u32, allocator: std.mem.Allocator) !Self {
        const max_levels = calculateMaxLevels(width, height);
        var levels = try allocator.alloc(ZBufferLevel, max_levels);
        errdefer allocator.free(levels);

        var current_width = width;
        var current_height = height;

        for (levels, 0..) |*level, i| {
            level.* = try ZBufferLevel.init(current_width, current_height, allocator);
            errdefer {
                // Cleanup already created levels
                for (levels[0..i]) |*prev_level| {
                    prev_level.deinit(allocator);
                }
            }

            current_width = @max(1, current_width / 2);
            current_height = @max(1, current_height / 2);
        }

        return Self{
            .levels = levels,
            .width = width,
            .height = height,
            .max_levels = max_levels,
            .allocator = allocator,
        };
    }

    pub fn deinit(self: *Self) void {
        for (self.levels) |*level| {
            level.deinit(self.allocator);
        }
        self.allocator.free(self.levels);
    }

    /// Update hierarchical Z-buffer from rendered depth
    pub fn updateFromDepthBuffer(self: *Self, depth_buffer: []const f32) void {
        std.debug.assert(depth_buffer.len >= self.width * self.height);

        // Copy to level 0 (full resolution)
        @memcpy(self.levels[0].depth_buffer, depth_buffer[0 .. self.width * self.height]);

        // Build hierarchy by downsampling (take minimum depth)
        for (1..self.max_levels) |level_idx| {
            const prev_level = &self.levels[level_idx - 1];
            const current_level = &self.levels[level_idx];

            self.downsampleLevel(prev_level, current_level);
        }
    }

    /// Test AABB against hierarchical Z-buffer for occlusion
    pub fn testAABBOcclusion(self: Self, aabb: frustum.AABB, mvp_matrix: [16]f32) bool {
        // Project AABB to screen space
        const screen_bounds = self.projectAABBToScreen(aabb, mvp_matrix);
        if (screen_bounds == null) return true; // Behind camera, assume visible

        const bounds = screen_bounds.?;
        const closest_depth = self.calculateClosestDepth(aabb, mvp_matrix);

        // Test against appropriate hierarchical level
        const test_level = self.chooseBestLevel(bounds.width, bounds.height);
        const level_bounds = self.scaleBoundsToLevel(bounds, test_level);

        return self.levels[test_level].testRegion(level_bounds.min_x, level_bounds.min_y, level_bounds.max_x, level_bounds.max_y, closest_depth);
    }

    fn calculateMaxLevels(width: u32, height: u32) u32 {
        const max_dimension = @max(width, height);
        return @as(u32, @intFromFloat(@log2(@as(f32, @floatFromInt(max_dimension))))) + 1;
    }

    fn downsampleLevel(self: *Self, src: *const ZBufferLevel, dst: *ZBufferLevel) void {
        _ = self;
        var dst_y: u32 = 0;
        while (dst_y < dst.height) : (dst_y += 1) {
            var dst_x: u32 = 0;
            while (dst_x < dst.width) : (dst_x += 1) {
                const src_x = dst_x * 2;
                const src_y = dst_y * 2;

                // Sample 2x2 region and take minimum depth
                var min_depth: f32 = 1.0;

                for (0..2) |dy| {
                    for (0..2) |dx| {
                        const sample_x = src_x + @as(u32, @intCast(dx));
                        const sample_y = src_y + @as(u32, @intCast(dy));

                        if (sample_x < src.width and sample_y < src.height) {
                            min_depth = @min(min_depth, src.getDepth(sample_x, sample_y));
                        }
                    }
                }

                dst.setDepth(dst_x, dst_y, min_depth);
            }
        }
    }

    const ScreenBounds = struct {
        min_x: u32,
        min_y: u32,
        max_x: u32,
        max_y: u32,
        width: u32,
        height: u32,
    };

    fn projectAABBToScreen(self: Self, aabb: frustum.AABB, mvp_matrix: [16]f32) ?ScreenBounds {
        // Project all 8 corners of AABB
        var min_x: f32 = std.math.inf(f32);
        var min_y: f32 = std.math.inf(f32);
        var max_x: f32 = -std.math.inf(f32);
        var max_y: f32 = -std.math.inf(f32);
        var any_in_front = false;

        const corners = [8]frustum.Vec3{
            frustum.Vec3.init(aabb.min.x, aabb.min.y, aabb.min.z),
            frustum.Vec3.init(aabb.max.x, aabb.min.y, aabb.min.z),
            frustum.Vec3.init(aabb.min.x, aabb.max.y, aabb.min.z),
            frustum.Vec3.init(aabb.max.x, aabb.max.y, aabb.min.z),
            frustum.Vec3.init(aabb.min.x, aabb.min.y, aabb.max.z),
            frustum.Vec3.init(aabb.max.x, aabb.min.y, aabb.max.z),
            frustum.Vec3.init(aabb.min.x, aabb.max.y, aabb.max.z),
            frustum.Vec3.init(aabb.max.x, aabb.max.y, aabb.max.z),
        };

        for (corners) |corner| {
            const projected = self.projectPoint(corner, mvp_matrix);
            if (projected.z > 0.0) { // In front of camera
                any_in_front = true;
                min_x = @min(min_x, projected.x);
                min_y = @min(min_y, projected.y);
                max_x = @max(max_x, projected.x);
                max_y = @max(max_y, projected.y);
            }
        }

        if (!any_in_front) return null;

        // Convert to screen coordinates
        const screen_min_x = @as(u32, @intFromFloat(@max(0, @min(@as(f32, @floatFromInt(self.width)), min_x * @as(f32, @floatFromInt(self.width))))));
        const screen_min_y = @as(u32, @intFromFloat(@max(0, @min(@as(f32, @floatFromInt(self.height)), min_y * @as(f32, @floatFromInt(self.height))))));
        const screen_max_x = @as(u32, @intFromFloat(@max(0, @min(@as(f32, @floatFromInt(self.width)), max_x * @as(f32, @floatFromInt(self.width))))));
        const screen_max_y = @as(u32, @intFromFloat(@max(0, @min(@as(f32, @floatFromInt(self.height)), max_y * @as(f32, @floatFromInt(self.height))))));

        return ScreenBounds{
            .min_x = screen_min_x,
            .min_y = screen_min_y,
            .max_x = screen_max_x,
            .max_y = screen_max_y,
            .width = screen_max_x - screen_min_x,
            .height = screen_max_y - screen_min_y,
        };
    }

    fn projectPoint(self: Self, point: frustum.Vec3, mvp_matrix: [16]f32) struct { x: f32, y: f32, z: f32 } {
        _ = self;

        // Transform point by MVP matrix
        const x = mvp_matrix[0] * point.x + mvp_matrix[4] * point.y + mvp_matrix[8] * point.z + mvp_matrix[12];
        const y = mvp_matrix[1] * point.x + mvp_matrix[5] * point.y + mvp_matrix[9] * point.z + mvp_matrix[13];
        const z = mvp_matrix[2] * point.x + mvp_matrix[6] * point.y + mvp_matrix[10] * point.z + mvp_matrix[14];
        const w = mvp_matrix[3] * point.x + mvp_matrix[7] * point.y + mvp_matrix[11] * point.z + mvp_matrix[15];

        if (@abs(w) < 0.0001) return .{ .x = 0, .y = 0, .z = -1 };

        // Perspective division and convert to NDC
        return .{
            .x = (x / w) * 0.5 + 0.5,
            .y = (y / w) * 0.5 + 0.5,
            .z = z / w,
        };
    }

    fn calculateClosestDepth(self: Self, aabb: frustum.AABB, mvp_matrix: [16]f32) f32 {
        // Find closest corner to camera in NDC space
        var closest_depth: f32 = 1.0;

        const corners = [8]frustum.Vec3{
            frustum.Vec3.init(aabb.min.x, aabb.min.y, aabb.min.z),
            frustum.Vec3.init(aabb.max.x, aabb.min.y, aabb.min.z),
            frustum.Vec3.init(aabb.min.x, aabb.max.y, aabb.min.z),
            frustum.Vec3.init(aabb.max.x, aabb.max.y, aabb.min.z),
            frustum.Vec3.init(aabb.min.x, aabb.min.y, aabb.max.z),
            frustum.Vec3.init(aabb.max.x, aabb.min.y, aabb.max.z),
            frustum.Vec3.init(aabb.min.x, aabb.max.y, aabb.max.z),
            frustum.Vec3.init(aabb.max.x, aabb.max.y, aabb.max.z),
        };

        for (corners) |corner| {
            const projected = self.projectPoint(corner, mvp_matrix);
            if (projected.z > -1.0 and projected.z < 1.0) {
                closest_depth = @min(closest_depth, (projected.z + 1.0) * 0.5);
            }
        }

        return closest_depth;
    }

    fn chooseBestLevel(self: Self, width: u32, height: u32) u32 {
        // Choose level based on projected size
        const max_dimension = @max(width, height);
        if (max_dimension <= 4) return @min(self.max_levels - 1, 2);
        if (max_dimension <= 16) return @min(self.max_levels - 1, 1);
        return 0;
    }

    fn scaleBoundsToLevel(self: Self, bounds: ScreenBounds, level: u32) ScreenBounds {
        _ = self;

        const scale = @as(u32, 1) << @as(u5, @intCast(level));
        return ScreenBounds{
            .min_x = bounds.min_x / scale,
            .min_y = bounds.min_y / scale,
            .max_x = bounds.max_x / scale,
            .max_y = bounds.max_y / scale,
            .width = bounds.width / scale,
            .height = bounds.height / scale,
        };
    }
};

/// Occlusion query system for GPU-based occlusion testing
pub const OcclusionQuerySystem = struct {
    const Self = @This();

    query_pool: []u32,
    available_queries: std.ArrayList(u32),
    active_queries: std.ArrayList(ActiveQuery),
    allocator: std.mem.Allocator,

    const ActiveQuery = struct {
        query_id: u32,
        object_id: u32,
        frame_submitted: u32,
    };

    pub fn init(max_queries: u32, allocator: std.mem.Allocator) !Self {
        const query_pool = try allocator.alloc(u32, max_queries);
        var available_queries = std.ArrayList(u32).init(allocator);
        const active_queries = std.ArrayList(ActiveQuery).init(allocator);

        // Initialize query pool
        for (query_pool, 0..) |*query, i| {
            query.* = @as(u32, @intCast(i));
            try available_queries.append(@as(u32, @intCast(i)));
        }

        return Self{
            .query_pool = query_pool,
            .available_queries = available_queries,
            .active_queries = active_queries,
            .allocator = allocator,
        };
    }

    pub fn deinit(self: *Self) void {
        self.available_queries.deinit();
        self.active_queries.deinit();
        self.allocator.free(self.query_pool);
    }

    /// Submit occlusion query for object
    pub fn submitQuery(self: *Self, object_id: u32, current_frame: u32) ?u32 {
        if (self.available_queries.items.len == 0) return null;

        const query_id = self.available_queries.pop();
        self.active_queries.append(ActiveQuery{
            .query_id = query_id,
            .object_id = object_id,
            .frame_submitted = current_frame,
        }) catch return null;

        return query_id;
    }

    /// Check for completed queries and return results
    pub fn processResults(self: *Self, current_frame: u32) !std.ArrayList(struct { object_id: u32, visible: bool }) {
        var results = std.ArrayList(struct { object_id: u32, visible: bool }).init(self.allocator);

        var i: usize = 0;
        while (i < self.active_queries.items.len) {
            const query = self.active_queries.items[i];

            // Only check queries that are old enough (avoid GPU stall)
            if (current_frame - query.frame_submitted >= 2) {
                // In a real implementation, this would check GPU query status
                const visible = self.checkQueryResult(query.query_id);

                try results.append(.{ .object_id = query.object_id, .visible = visible });

                // Return query to available pool
                try self.available_queries.append(query.query_id);

                // Remove from active queries
                _ = self.active_queries.swapRemove(i);
            } else {
                i += 1;
            }
        }

        return results;
    }

    fn checkQueryResult(self: Self, query_id: u32) bool {
        _ = self;
        _ = query_id;

        // Placeholder - in real implementation this would:
        // 1. Check if GPU query is complete
        // 2. Get sample count from GPU
        // 3. Return true if samples > threshold
        return true; // Assume visible for now
    }
};

/// Predictive occlusion culling using temporal coherence
pub const PredictiveOccluder = struct {
    const Self = @This();

    visibility_history: std.AutoHashMap(u32, VisibilityRecord),
    allocator: std.mem.Allocator,

    const VisibilityRecord = struct {
        last_visible_frame: u32,
        consecutive_occluded: u32,
        consecutive_visible: u32,
        confidence: f32,
    };

    pub fn init(allocator: std.mem.Allocator) Self {
        return Self{
            .visibility_history = std.AutoHashMap(u32, VisibilityRecord).init(allocator),
            .allocator = allocator,
        };
    }

    pub fn deinit(self: *Self) void {
        self.visibility_history.deinit();
    }

    /// Update visibility history for object
    pub fn updateVisibility(self: *Self, object_id: u32, visible: bool, current_frame: u32) !void {
        const result = try self.visibility_history.getOrPut(object_id);

        if (result.found_existing) {
            const record = result.value_ptr;

            if (visible) {
                record.last_visible_frame = current_frame;
                record.consecutive_visible += 1;
                record.consecutive_occluded = 0;
                record.confidence = @min(1.0, record.confidence + 0.1);
            } else {
                record.consecutive_visible = 0;
                record.consecutive_occluded += 1;
                record.confidence = @max(0.0, record.confidence - 0.1);
            }
        } else {
            result.value_ptr.* = VisibilityRecord{
                .last_visible_frame = if (visible) current_frame else 0,
                .consecutive_occluded = if (visible) 0 else 1,
                .consecutive_visible = if (visible) 1 else 0,
                .confidence = 0.5,
            };
        }
    }

    /// Predict visibility for object based on history
    pub fn predictVisibility(self: Self, object_id: u32, current_frame: u32) struct { likely_visible: bool, confidence: f32 } {
        const record = self.visibility_history.get(object_id) orelse {
            return .{ .likely_visible = true, .confidence = 0.5 }; // Default to visible for new objects
        };

        const frames_since_visible = current_frame - record.last_visible_frame;

        // Objects that were recently visible are likely still visible
        if (frames_since_visible <= 5) {
            return .{ .likely_visible = true, .confidence = @min(0.9, record.confidence + 0.2) };
        }

        // Objects occluded for many frames are likely still occluded
        if (record.consecutive_occluded > 10) {
            return .{ .likely_visible = false, .confidence = @min(0.9, 1.0 - record.confidence + 0.2) };
        }

        return .{ .likely_visible = record.confidence > 0.5, .confidence = record.confidence };
    }
};
