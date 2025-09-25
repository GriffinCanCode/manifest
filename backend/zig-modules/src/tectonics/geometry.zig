//! Tectonic Geometry Calculations
//!
//! High-performance SIMD-optimized spatial geometry calculations for
//! boundary detection, distance calculations, and collision detection.

const std = @import("std");

const simd = @import("../simd/simd.zig");

/// 2D Point for geometric calculations
pub const Point2D = struct {
    x: f64,
    y: f64,

    pub fn init(x: f64, y: f64) Point2D {
        return Point2D{ .x = x, .y = y };
    }

    pub fn distance(self: Point2D, other: Point2D) f64 {
        const dx = self.x - other.x;
        const dy = self.y - other.y;
        return @sqrt(dx * dx + dy * dy);
    }

    pub fn distanceSquared(self: Point2D, other: Point2D) f64 {
        const dx = self.x - other.x;
        const dy = self.y - other.y;
        return dx * dx + dy * dy;
    }
};

/// Line segment for boundary calculations
pub const LineSegment = struct {
    start: Point2D,
    end: Point2D,

    pub fn init(start: Point2D, end: Point2D) LineSegment {
        return LineSegment{ .start = start, .end = end };
    }

    pub fn length(self: LineSegment) f64 {
        return self.start.distance(self.end);
    }

    pub fn lengthSquared(self: LineSegment) f64 {
        return self.start.distanceSquared(self.end);
    }
};

/// Polygon for plate boundary representation
pub const Polygon = struct {
    vertices: []const Point2D,

    pub fn init(vertices: []const Point2D) Polygon {
        return Polygon{ .vertices = vertices };
    }

    pub fn perimeter(self: Polygon) f64 {
        if (self.vertices.len < 2) return 0.0;

        var total: f64 = 0.0;
        for (0..self.vertices.len) |i| {
            const next_i = (i + 1) % self.vertices.len;
            total += self.vertices[i].distance(self.vertices[next_i]);
        }
        return total;
    }

    pub fn area(self: Polygon) f64 {
        if (self.vertices.len < 3) return 0.0;

        var area_sum: f64 = 0.0;
        for (0..self.vertices.len) |i| {
            const next_i = (i + 1) % self.vertices.len;
            const curr = self.vertices[i];
            const next = self.vertices[next_i];
            area_sum += curr.x * next.y - next.x * curr.y;
        }
        return @abs(area_sum) / 2.0;
    }

    pub fn centroid(self: Polygon) Point2D {
        if (self.vertices.len == 0) return Point2D.init(0.0, 0.0);

        var sum_x: f64 = 0.0;
        var sum_y: f64 = 0.0;

        for (self.vertices) |vertex| {
            sum_x += vertex.x;
            sum_y += vertex.y;
        }

        const count = @as(f64, @floatFromInt(self.vertices.len));
        return Point2D.init(sum_x / count, sum_y / count);
    }

    /// Check if point is inside polygon using ray casting algorithm
    pub fn containsPoint(self: Polygon, point: Point2D) bool {
        if (self.vertices.len < 3) return false;

        var inside = false;
        var j = self.vertices.len - 1;

        for (self.vertices, 0..) |vertex_i, i| {
            const vertex_j = self.vertices[j];

            if (((vertex_i.y > point.y) != (vertex_j.y > point.y)) and
                (point.x < (vertex_j.x - vertex_i.x) * (point.y - vertex_i.y) / (vertex_j.y - vertex_i.y) + vertex_i.x))
            {
                inside = !inside;
            }
            j = i;
        }

        return inside;
    }
};

/// Calculate minimum distance from point to line segment
pub fn pointToSegmentDistance(point: Point2D, segment: LineSegment) f64 {
    const segment_length_sq = segment.lengthSquared();
    if (segment_length_sq < 1e-10) {
        return point.distance(segment.start);
    }

    const dx = segment.end.x - segment.start.x;
    const dy = segment.end.y - segment.start.y;
    const px = point.x - segment.start.x;
    const py = point.y - segment.start.y;

    const t = @max(0.0, @min(1.0, (px * dx + py * dy) / segment_length_sq));

    const projection = Point2D.init(
        segment.start.x + t * dx,
        segment.start.y + t * dy,
    );

    return point.distance(projection);
}

/// Calculate minimum distance between two line segments
pub fn segmentToSegmentDistance(seg1: LineSegment, seg2: LineSegment) f64 {
    // Check all point-to-segment distances
    const distances = [4]f64{
        pointToSegmentDistance(seg1.start, seg2),
        pointToSegmentDistance(seg1.end, seg2),
        pointToSegmentDistance(seg2.start, seg1),
        pointToSegmentDistance(seg2.end, seg1),
    };

    var min_dist = distances[0];
    for (distances[1..]) |dist| {
        if (dist < min_dist) min_dist = dist;
    }

    return min_dist;
}

/// Check if two line segments intersect
pub fn segmentsIntersect(seg1: LineSegment, seg2: LineSegment) bool {
    const d1 = orientation(seg2.start, seg2.end, seg1.start);
    const d2 = orientation(seg2.start, seg2.end, seg1.end);
    const d3 = orientation(seg1.start, seg1.end, seg2.start);
    const d4 = orientation(seg1.start, seg1.end, seg2.end);

    if (((d1 > 0 and d2 < 0) or (d1 < 0 and d2 > 0)) and
        ((d3 > 0 and d4 < 0) or (d3 < 0 and d4 > 0)))
    {
        return true;
    }

    // Check for collinear cases
    if (d1 == 0 and onSegment(seg2.start, seg1.start, seg2.end)) return true;
    if (d2 == 0 and onSegment(seg2.start, seg1.end, seg2.end)) return true;
    if (d3 == 0 and onSegment(seg1.start, seg2.start, seg1.end)) return true;
    if (d4 == 0 and onSegment(seg1.start, seg2.end, seg1.end)) return true;

    return false;
}

/// Calculate orientation of ordered triplet of points
fn orientation(p: Point2D, q: Point2D, r: Point2D) f64 {
    return (q.y - p.y) * (r.x - q.x) - (q.x - p.x) * (r.y - q.y);
}

/// Check if point q lies on line segment pr
fn onSegment(p: Point2D, q: Point2D, r: Point2D) bool {
    return q.x <= @max(p.x, r.x) and q.x >= @min(p.x, r.x) and
        q.y <= @max(p.y, r.y) and q.y >= @min(p.y, r.y);
}

/// Calculate minimum distance between two polygons
pub fn polygonToPolygonDistance(poly1: Polygon, poly2: Polygon) f64 {
    if (poly1.vertices.len < 2 or poly2.vertices.len < 2) {
        return std.math.inf(f64);
    }

    var min_distance = std.math.inf(f64);

    // Check all segment pairs
    for (0..poly1.vertices.len) |i| {
        const next_i = (i + 1) % poly1.vertices.len;
        const seg1 = LineSegment.init(poly1.vertices[i], poly1.vertices[next_i]);

        for (0..poly2.vertices.len) |j| {
            const next_j = (j + 1) % poly2.vertices.len;
            const seg2 = LineSegment.init(poly2.vertices[j], poly2.vertices[next_j]);

            const distance = segmentToSegmentDistance(seg1, seg2);
            if (distance < min_distance) {
                min_distance = distance;
            }
        }
    }

    return min_distance;
}

/// Check if two polygons intersect
pub fn polygonsIntersect(poly1: Polygon, poly2: Polygon) bool {
    // Check if any edges intersect
    for (0..poly1.vertices.len) |i| {
        const next_i = (i + 1) % poly1.vertices.len;
        const seg1 = LineSegment.init(poly1.vertices[i], poly1.vertices[next_i]);

        for (0..poly2.vertices.len) |j| {
            const next_j = (j + 1) % poly2.vertices.len;
            const seg2 = LineSegment.init(poly2.vertices[j], poly2.vertices[next_j]);

            if (segmentsIntersect(seg1, seg2)) {
                return true;
            }
        }
    }

    // Check if one polygon is inside the other
    if (poly1.vertices.len > 0 and poly2.containsPoint(poly1.vertices[0])) {
        return true;
    }
    if (poly2.vertices.len > 0 and poly1.containsPoint(poly2.vertices[0])) {
        return true;
    }

    return false;
}

/// Batch distance calculations between points using SIMD when possible
pub fn batchPointDistances(
    points1: []const Point2D,
    points2: []const Point2D,
    distances: []f64,
) void {
    std.debug.assert(distances.len >= points1.len * points2.len);

    for (points1, 0..) |p1, i| {
        for (points2, 0..) |p2, j| {
            const index = i * points2.len + j;
            distances[index] = p1.distance(p2);
        }
    }
}

/// Find closest point on polygon boundary to given point
pub fn closestPointOnPolygon(polygon: Polygon, point: Point2D) Point2D {
    if (polygon.vertices.len == 0) return point;
    if (polygon.vertices.len == 1) return polygon.vertices[0];

    var closest_point = polygon.vertices[0];
    var min_distance = point.distance(polygon.vertices[0]);

    // Check all edges
    for (0..polygon.vertices.len) |i| {
        const next_i = (i + 1) % polygon.vertices.len;
        const segment = LineSegment.init(polygon.vertices[i], polygon.vertices[next_i]);

        const segment_length_sq = segment.lengthSquared();
        if (segment_length_sq < 1e-10) continue;

        const dx = segment.end.x - segment.start.x;
        const dy = segment.end.y - segment.start.y;
        const px = point.x - segment.start.x;
        const py = point.y - segment.start.y;

        const t = @max(0.0, @min(1.0, (px * dx + py * dy) / segment_length_sq));

        const projection = Point2D.init(
            segment.start.x + t * dx,
            segment.start.y + t * dy,
        );

        const distance = point.distance(projection);
        if (distance < min_distance) {
            min_distance = distance;
            closest_point = projection;
        }
    }

    return closest_point;
}

/// Calculate convex hull using Graham scan algorithm
pub fn convexHull(points: []const Point2D, hull: []Point2D, allocator: std.mem.Allocator) !usize {
    if (points.len < 3) return 0;

    // Find bottom-most point (or left-most in case of tie)
    var bottom_point = points[0];
    var bottom_index: usize = 0;

    for (points, 0..) |point, i| {
        if (point.y < bottom_point.y or (point.y == bottom_point.y and point.x < bottom_point.x)) {
            bottom_point = point;
            bottom_index = i;
        }
    }

    // Create sorted array by polar angle
    var sorted_points = try allocator.alloc(Point2D, points.len);
    defer allocator.free(sorted_points);

    for (points, 0..) |point, i| {
        sorted_points[i] = point;
    }

    // Swap bottom point to first position
    sorted_points[0] = sorted_points[bottom_index];
    sorted_points[bottom_index] = points[0];

    // Sort by polar angle
    const Context = struct {
        bottom: Point2D,

        pub fn lessThan(ctx: @This(), a: Point2D, b: Point2D) bool {
            const o = orientation(ctx.bottom, a, b);
            if (o == 0) {
                return ctx.bottom.distanceSquared(a) < ctx.bottom.distanceSquared(b);
            }
            return o > 0;
        }
    };

    std.sort.insertion(Point2D, sorted_points[1..], Context{ .bottom = bottom_point }, Context.lessThan);

    // Build convex hull
    var hull_size: usize = 0;
    hull[hull_size] = sorted_points[0];
    hull_size += 1;
    hull[hull_size] = sorted_points[1];
    hull_size += 1;

    for (sorted_points[2..]) |point| {
        // Remove points that make clockwise turn
        while (hull_size > 1 and orientation(hull[hull_size - 2], hull[hull_size - 1], point) <= 0) {
            hull_size -= 1;
        }
        hull[hull_size] = point;
        hull_size += 1;
    }

    return hull_size;
}

/// Calculate area of triangle formed by three points
pub fn triangleArea(p1: Point2D, p2: Point2D, p3: Point2D) f64 {
    return @abs((p1.x * (p2.y - p3.y) + p2.x * (p3.y - p1.y) + p3.x * (p1.y - p2.y)) / 2.0);
}

/// Check if point is inside triangle
pub fn pointInTriangle(point: Point2D, p1: Point2D, p2: Point2D, p3: Point2D) bool {
    const area_original = triangleArea(p1, p2, p3);
    const area1 = triangleArea(point, p2, p3);
    const area2 = triangleArea(p1, point, p3);
    const area3 = triangleArea(p1, p2, point);

    const epsilon = 1e-10;
    return @abs(area_original - (area1 + area2 + area3)) < epsilon;
}

/// Batch polygon containment tests
pub fn batchContainmentTests(
    polygons: []const Polygon,
    points: []const Point2D,
    results: []bool,
) void {
    std.debug.assert(results.len >= polygons.len * points.len);

    for (polygons, 0..) |polygon, i| {
        for (points, 0..) |point, j| {
            const index = i * points.len + j;
            results[index] = polygon.containsPoint(point);
        }
    }
}
