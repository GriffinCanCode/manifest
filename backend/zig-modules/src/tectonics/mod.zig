//! Tectonics Module - Comprehensive Geological Simulation
//!
//! High-performance SIMD-optimized tectonic plate physics, geological stress analysis,
//! volcanic activity modeling, and geometric calculations for realistic world generation.
//!
//! ## Features
//! - Tectonic plate physics with ridge push, slab pull, and mantle convection
//! - 2D stress field analysis with principal stress calculations
//! - Volcanic hazard assessment and eruption modeling
//! - Advanced geometric calculations for plate boundaries
//! - SIMD-optimized batch processing for performance
//!
//! ## Usage
//! ```zig
//! const tectonics = @import("tectonics/mod.zig");
//!
//! // Create and simulate tectonic plates
//! const plate = tectonics.plates.TectonicPlate{ ... };
//! const ridge_force = tectonics.plates.calculateRidgePush(&plate, movement_speed);
//!
//! // Calculate stress fields
//! const stress_tensor = tectonics.stress.StressTensor.init(xx, yy, xy);
//! const von_mises = stress_tensor.vonMisesStress();
//!
//! // Volcanic hazard assessment
//! const volcano = tectonics.volcanic.Volcano{ ... };
//! const hazard = tectonics.volcanic.calculatePyroclasticFlowHazard(&volcano, x, y, wind_dir, wind_speed);
//! ```

pub const geometry = @import("geometry.zig");
pub const Point2D = geometry.Point2D;
pub const LineSegment = geometry.LineSegment;
pub const Polygon = geometry.Polygon;
pub const Circle = geometry.Circle;
pub const BoundingBox = geometry.BoundingBox;
pub const pointToSegmentDistance = geometry.pointToSegmentDistance;
pub const lineIntersection = geometry.lineIntersection;
pub const polygonContainsPoint = geometry.polygonContainsPoint;
pub const batchPointDistances = geometry.batchPointDistances;
pub const plates = @import("plates.zig");
pub const Vec2 = plates.Vec2;
pub const TectonicPlate = plates.TectonicPlate;
pub const PlateInteraction = plates.PlateInteraction;
pub const InteractionType = plates.InteractionType;
pub const calculateRidgePush = plates.calculateRidgePush;
pub const calculateSlabPull = plates.calculateSlabPull;
pub const calculateBasalDrag = plates.calculateBasalDrag;
pub const calculateMantelConvection = plates.calculateMantelConvection;
pub const updatePlateVelocity = plates.updatePlateVelocity;
pub const batchDistanceCalculations = plates.batchDistanceCalculations;
pub const stress = @import("stress.zig");
pub const StressTensor = stress.StressTensor;
pub const StressField = stress.StressField;
pub const SeismicHazard = stress.SeismicHazard;
pub const FaultSystem = stress.FaultSystem;
pub const FaultSegment = stress.FaultSegment;
pub const volcanic = @import("volcanic.zig");
pub const Volcano = volcanic.Volcano;
pub const VolcanicHazard = volcanic.VolcanicHazard;
pub const MagmaChamber = volcanic.MagmaChamber;
pub const EruptionType = volcanic.EruptionType;
pub const calculatePyroclasticFlowHazard = volcanic.calculatePyroclasticFlowHazard;
pub const calculateAshFallHazard = volcanic.calculateAshFallHazard;
pub const calculateLavaFlowHazard = volcanic.calculateLavaFlowHazard;

// Re-export commonly used geometry types
// Re-export plate physics types
// Re-export stress analysis types
// Re-export volcanic types
// Common geometric operations
