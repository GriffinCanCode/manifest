//! Climate System Module
//!
//! Comprehensive SIMD-optimized climate system for game world generation.
//! Handles orographic effects, continental effects, seasonal variations,
//! and advanced climate interpolation.
//!
//! ## Features
//! - Orographic rainfall enhancement and rain shadow effects
//! - Continental temperature and humidity modulation
//! - Seasonal climate variations with hemisphere support
//! - Advanced climate interpolation and smoothing
//! - Batch processing for optimal performance
//!
//! ## Usage
//! ```zig
//! const climate = @import("climate/mod.zig");
//!
//! // Simple climate processing
//! climate.simpleClimateProcessing(
//!     positions, elevations, base_temps, base_rainfall, base_humidity,
//!     wind_directions, temp_results, rain_results, hum_results
//! );
//!
//! // Advanced processing with custom parameters
//! const params = climate.ClimateProcessingParams.default();
//! climate.processClimateEffects(...);
//! ```

const climate_main = @import("climate.zig");
pub const ClimateProcessingParams = climate_main.ClimateProcessingParams;
pub const processClimateEffects = climate_main.processClimateEffects;
pub const simpleClimateProcessing = climate_main.simpleClimateProcessing;
pub const processOrographicOnly = climate_main.processOrographicOnly;
pub const processContinentalOnly = climate_main.processContinentalOnly;
pub const continental = @import("continental.zig");
pub const ContinentalParams = continental.ContinentalParams;
pub const interpolation = @import("interpolation.zig");
pub const ClimateData = interpolation.ClimateData;
pub const InterpolationParams = interpolation.InterpolationParams;
pub const orographic = @import("orographic.zig");
pub const OrographicParams = orographic.OrographicParams;
pub const MountainRange = orographic.MountainRange;
pub const seasonal = @import("seasonal.zig");
pub const ClimateZone = seasonal.ClimateZone;
pub const SeasonalParams = seasonal.SeasonalParams;
pub const SeasonalState = seasonal.SeasonalState;

// Re-export submodules for direct access
// Re-export main functions from climate.zig
// Re-export important types for convenience
