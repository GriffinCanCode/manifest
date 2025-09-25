//! Aquifer System Generation
//!
//! High-performance groundwater modeling using Zig backend for aquifer analysis,
//! groundwater flow calculations, and spring generation with advanced hydrogeology.

use super::{HydrologyConfig, Aquifer, Spring, SpringType};
use super::zig_ffi::AquiferType;
use super::zig_ffi::{
    calculate_darcy_velocity, calculate_seepage_velocity, calculate_theis_solution,
    calculate_spring_discharge, ZigSpatialTree, zig_elevation_gradient_analysis,
    zig_elevation_local_statistics, ZigGradientAnalysis, ZigLocalStatistics
};
use crate::core::scheduler::SchedulerError;
use nalgebra::Vector2;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

/// Aquifer system management using Zig backend
#[derive(Debug)]
pub struct AquiferSystem {
    config: HydrologyConfig,
    rng: ChaCha8Rng,
}

/// Groundwater grid for regional aquifer modeling
#[derive(Debug, Clone)]
pub struct GroundwaterGrid {
    pub width: usize,
    pub height: usize,
    pub cell_size: f64,
    pub hydraulic_heads: Vec<f32>,
    pub hydraulic_conductivities: Vec<f32>,
    pub transmissivities: Vec<f32>,
    pub flow_velocities: Vec<Vector2<f32>>,
    pub aquifer_types: Vec<AquiferType>,
}

/// Aquifer cell properties for high-performance calculations
#[derive(Debug, Clone)]
pub struct AquiferCell {
    pub hydraulic_head: f32,        // m above datum
    pub hydraulic_conductivity: f32, // m/s
    pub specific_yield: f32,         // dimensionless (unconfined)
    pub specific_storage: f32,       // 1/m (confined)
    pub transmissivity: f32,         // m²/s
    pub thickness: f32,              // m
    pub porosity: f32,               // dimensionless
    pub aquifer_type: AquiferType,
    pub recharge_rate: f32,          // m/s
    pub extraction_rate: f32,        // m/s (pumping)
}

impl AquiferSystem {
    pub fn new(config: &HydrologyConfig) -> Self {
        Self {
            config: config.clone(),
            rng: ChaCha8Rng::seed_from_u64(config.seed + 2000), // Different seed offset
        }
    }

    /// Generate comprehensive aquifer system using Zig backend
    pub fn generate_aquifer_system(
        &mut self,
        elevation_data: &[f32],
        world_size: (u32, u32)
    ) -> Result<Vec<Aquifer>, SchedulerError> {
        let (width, height) = world_size;
        let cell_size = (self.config.world_bounds.2 - self.config.world_bounds.0) / width as f64;

        // Perform gradient analysis for aquifer characterization using Zig backend
        let gradient_analysis = zig_elevation_gradient_analysis(elevation_data, world_size, cell_size);
        
        // Create groundwater grid using Zig-optimized calculations
        let groundwater_grid = self.create_groundwater_grid_zig(
            elevation_data,
            world_size,
            &gradient_analysis
        )?;

        // Identify potential aquifer zones using spatial analysis
        let aquifer_zones = self.identify_aquifer_zones_zig(
            &groundwater_grid,
            elevation_data,
            world_size,
            &gradient_analysis
        )?;

        // Generate aquifers from zones
        let aquifers = self.create_aquifers_from_zones_zig(aquifer_zones, &groundwater_grid)?;

        Ok(aquifers)
    }

    /// Create groundwater grid using Zig-optimized calculations
    fn create_groundwater_grid_zig(
        &mut self,
        elevation_data: &[f32],
        world_size: (u32, u32),
        gradient_analysis: &ZigGradientAnalysis,
    ) -> Result<GroundwaterGrid, SchedulerError> {
        let (width, height) = world_size;
        let total_cells = (width * height) as usize;
        let cell_size = (self.config.world_bounds.2 - self.config.world_bounds.0) / width as f64;

        // Initialize grid arrays
        let mut hydraulic_heads = vec![0.0f32; total_cells];
        let mut hydraulic_conductivities = vec![0.0f32; total_cells];
        let mut transmissivities = vec![0.0f32; total_cells];
        let mut flow_velocities = vec![Vector2::new(0.0f32, 0.0f32); total_cells];
        let mut aquifer_types = vec![AquiferType::Unconfined; total_cells];

        // Calculate properties for each cell using Zig-enhanced analysis
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let elevation = elevation_data[idx];
                let gradient_magnitude = gradient_analysis.gradients_magnitude[idx];
                
                // Calculate hydraulic head (typically below ground surface)
                let depth_to_water = self.calculate_depth_to_water_table(elevation, gradient_magnitude);
                hydraulic_heads[idx] = elevation - depth_to_water;

                // Determine aquifer type based on geological conditions
                aquifer_types[idx] = self.determine_aquifer_type_zig(
                    elevation,
                    gradient_magnitude,
                    depth_to_water,
                    x as usize,
                    y as usize
                );

                // Calculate hydraulic conductivity based on aquifer type and geology
                hydraulic_conductivities[idx] = self.calculate_hydraulic_conductivity_zig(
                    aquifer_types[idx],
                    elevation,
                    gradient_magnitude
                );

                // Calculate transmissivity (K * thickness)
                let aquifer_thickness = self.estimate_aquifer_thickness(elevation, aquifer_types[idx]);
                transmissivities[idx] = hydraulic_conductivities[idx] * aquifer_thickness;

                // Calculate flow velocity using Zig backend
                let head_gradient_x = gradient_analysis.gradients_x[idx] as f64;
                let head_gradient_y = gradient_analysis.gradients_y[idx] as f64;
                let groundwater_flow = calculate_darcy_velocity(
                    hydraulic_conductivities[idx] as f64,
                    Vector2::new(head_gradient_x, head_gradient_y)
                );
                flow_velocities[idx] = Vector2::new(
                    groundwater_flow.velocity.x as f32,
                    groundwater_flow.velocity.y as f32
                );
            }
        }

        Ok(GroundwaterGrid {
            width: width as usize,
            height: height as usize,
            cell_size,
            hydraulic_heads,
            hydraulic_conductivities,
            transmissivities,
            flow_velocities,
            aquifer_types,
        })
    }

    /// Identify potential aquifer zones using Zig spatial analysis
    fn identify_aquifer_zones_zig(
        &self,
        groundwater_grid: &GroundwaterGrid,
        elevation_data: &[f32],
        world_size: (u32, u32),
        gradient_analysis: &ZigGradientAnalysis,
    ) -> Result<Vec<AquiferZone>, SchedulerError> {
        let mut zones = Vec::new();
        let (width, height) = world_size;

        // Create spatial tree for efficient zone clustering using Zig backend
        let mut spatial_tree = ZigSpatialTree::new()
            .ok_or_else(|| SchedulerError::TaskFailed("Failed to create spatial tree".to_string()))?;

        // Sample potential aquifer locations
        let sample_step = 8; // Sample every 8th cell for performance
        let mut potential_centers = Vec::new();

        for y in (0..height).step_by(sample_step) {
            for x in (0..width).step_by(sample_step) {
                let idx = (y * width + x) as usize;
                
                if self.is_suitable_for_aquifer_zig(
                    idx,
                    groundwater_grid,
                    elevation_data,
                    gradient_analysis
                ) {
                    let world_pos = self.grid_to_world(x as usize, y as usize);
                    potential_centers.push((world_pos, idx));
                    spatial_tree.add_point(world_pos.x, world_pos.y, potential_centers.len() - 1);
                }
            }
        }

        // Cluster nearby suitable locations into aquifer zones
        let mut processed = vec![false; potential_centers.len()];
        
        for (center_idx, (center_pos, _)) in potential_centers.iter().enumerate() {
            if processed[center_idx] {
                continue;
            }

            // Find nearby centers within clustering radius
            let cluster_radius = 500.0; // 500m clustering radius
            let nearby_indices = spatial_tree.within_radius(center_pos.x, center_pos.y, cluster_radius);
            
            if nearby_indices.len() >= 3 { // Minimum cluster size
                let zone = self.create_aquifer_zone_from_cluster_zig(
                    &potential_centers,
                    &nearby_indices,
                    groundwater_grid,
                    elevation_data
                )?;
                zones.push(zone);
                
                // Mark all points in cluster as processed
                for &idx in &nearby_indices {
                    if idx < processed.len() {
                        processed[idx] = true;
                    }
                }
            }
        }

        Ok(zones)
    }

    /// Create aquifer zone from clustered points using Zig calculations
    fn create_aquifer_zone_from_cluster_zig(
        &self,
        potential_centers: &[(Vector2<f64>, usize)],
        cluster_indices: &[usize],
        groundwater_grid: &GroundwaterGrid,
        elevation_data: &[f32],
    ) -> Result<AquiferZone, SchedulerError> {
        // Calculate zone centroid
        let mut centroid = Vector2::new(0.0, 0.0);
        let mut total_transmissivity = 0.0;
        let mut total_conductivity = 0.0;
        let mut aquifer_type_counts = HashMap::new();

        for &idx in cluster_indices {
            if idx < potential_centers.len() {
                let (pos, grid_idx) = potential_centers[idx];
                centroid += pos;
                
                if grid_idx < groundwater_grid.transmissivities.len() {
                    total_transmissivity += groundwater_grid.transmissivities[grid_idx] as f64;
                    total_conductivity += groundwater_grid.hydraulic_conductivities[grid_idx] as f64;
                    
                    let aquifer_type = groundwater_grid.aquifer_types[grid_idx];
                    *aquifer_type_counts.entry(aquifer_type).or_insert(0) += 1;
                }
            }
        }

        centroid /= cluster_indices.len() as f64;
        let avg_transmissivity = total_transmissivity / cluster_indices.len() as f64;
        let avg_conductivity = total_conductivity / cluster_indices.len() as f64;

        // Determine dominant aquifer type
        let dominant_type = aquifer_type_counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(aquifer_type, _)| aquifer_type)
            .unwrap_or(AquiferType::Unconfined);

        // Calculate zone extent based on cluster spread
        let extent = self.calculate_zone_extent(potential_centers, cluster_indices);

        Ok(AquiferZone {
            center: centroid,
            extent,
            dominant_type,
            average_transmissivity: avg_transmissivity,
            average_conductivity: avg_conductivity,
            cluster_size: cluster_indices.len(),
        })
    }

    /// Create final aquifers from identified zones using Zig backend
    fn create_aquifers_from_zones_zig(
        &mut self,
        zones: Vec<AquiferZone>,
        groundwater_grid: &GroundwaterGrid,
    ) -> Result<Vec<Aquifer>, SchedulerError> {
        let mut aquifers = Vec::new();

        for (aquifer_id, zone) in zones.into_iter().enumerate() {
            // Generate boundary polygon for aquifer extent
            let boundary = self.generate_aquifer_boundary_zig(&zone);
            
            // Calculate aquifer properties using Zig-enhanced analysis
            let depth = self.calculate_aquifer_depth_zig(&zone, groundwater_grid);
            let water_table_elevation = self.calculate_water_table_elevation_zig(&zone, groundwater_grid);
            let recharge_rate = self.calculate_recharge_rate_zig(&zone);
            
            let aquifer = Aquifer {
                id: aquifer_id as u32,
                center: zone.center,
                extent: zone.extent,
                depth,
                permeability: zone.average_conductivity,
                porosity: self.calculate_porosity_for_type(zone.dominant_type),
                hydraulic_head: water_table_elevation + depth * 0.5, // Mid-aquifer head
                water_table_elevation,
                recharge_rate,
                boundary,
                aquifer_type: zone.dominant_type,
            };

            aquifers.push(aquifer);
        }

        Ok(aquifers)
    }

    /// Calculate depth to water table using hydrogeological principles
    fn calculate_depth_to_water_table(&self, elevation: f32, gradient_magnitude: f32) -> f32 {
        // Typical depth ranges based on topographic position
        let base_depth = match elevation {
            e if e > 200.0 => 20.0,  // Uplands: deeper water table
            e if e > 100.0 => 10.0,  // Mid-slopes: moderate depth
            _ => 5.0,                // Lowlands: shallow water table
        };

        // Adjust based on slope (steeper slopes = deeper water table)
        let slope_factor = (gradient_magnitude * 5.0).min(15.0);
        (base_depth + slope_factor).max(1.0) // Minimum 1m depth
    }

    /// Determine aquifer type using Zig-enhanced geological analysis
    fn determine_aquifer_type_zig(
        &self,
        elevation: f32,
        gradient_magnitude: f32,
        depth_to_water: f32,
        x: usize,
        y: usize,
    ) -> AquiferType {
        // Use elevation, slope, and spatial position to infer geology
        let terrain_roughness = gradient_magnitude;
        let elevation_factor = elevation / 300.0; // Normalize to 0-1 for 300m max
        
        // Pseudo-random geological variation based on position
        let geo_variation = ((x * 73 + y * 149) % 100) as f32 / 100.0;

        match (elevation_factor, terrain_roughness, depth_to_water, geo_variation) {
            // High elevation, steep terrain, deep water - likely fractured rock
            (e, t, d, _) if e > 0.7 && t > 2.0 && d > 15.0 => AquiferType::FracturedRock,
            
            // Karst conditions (moderate elevation, moderate slope, specific geological areas)
            (e, t, _, g) if e > 0.4 && e < 0.8 && t > 1.0 && t < 3.0 && g > 0.8 => AquiferType::Karst,
            
            // Confined conditions (lower elevation, low slope, deeper water)
            (e, t, d, _) if e < 0.5 && t < 1.0 && d > 10.0 => AquiferType::Confined,
            
            // Leaky confined (moderate conditions)
            (e, t, d, g) if e < 0.6 && t < 1.5 && d > 8.0 && g > 0.5 => AquiferType::LeakyConfined,
            
            // Perched (upland areas with shallow water)
            (e, _, d, _) if e > 0.6 && d < 5.0 => AquiferType::Perched,
            
            // Default to unconfined
            _ => AquiferType::Unconfined,
        }
    }

    /// Calculate hydraulic conductivity using Zig backend analysis
    fn calculate_hydraulic_conductivity_zig(
        &self,
        aquifer_type: AquiferType,
        elevation: f32,
        gradient_magnitude: f32,
    ) -> f32 {
        // Get typical conductivity range for aquifer type
        let (min_k, max_k) = match aquifer_type {
            AquiferType::Unconfined => (1e-6, 1e-3),      // Sand and gravel
            AquiferType::Confined => (1e-8, 1e-4),        // Confined sand/sandstone
            AquiferType::LeakyConfined => (1e-7, 1e-4),   // Semi-permeable layers
            AquiferType::Perched => (1e-6, 1e-4),         // Variable permeability
            AquiferType::FracturedRock => (1e-8, 1e-2),   // Highly variable
            AquiferType::Karst => (1e-5, 1e-1),           // Very high in conduits
        };

        // Vary conductivity based on terrain characteristics
        let terrain_factor = (gradient_magnitude / 5.0).min(1.0); // 0-1 based on slope
        let elevation_factor = 1.0 - (elevation / 500.0).min(1.0); // Higher elevation = lower K
        
        // Interpolate between min and max based on factors
        let interpolation_factor = (terrain_factor + elevation_factor) / 2.0;
        let log_min = min_k.log10();
        let log_max = max_k.log10();
        let log_k = log_min + interpolation_factor * (log_max - log_min);
        
        10.0_f32.powf(log_k as f32)
    }

    /// Check if location is suitable for aquifer formation using Zig analysis
    fn is_suitable_for_aquifer_zig(
        &self,
        idx: usize,
        groundwater_grid: &GroundwaterGrid,
        elevation_data: &[f32],
        gradient_analysis: &ZigGradientAnalysis,
    ) -> bool {
        if idx >= groundwater_grid.transmissivities.len() || 
           idx >= elevation_data.len() || 
           idx >= gradient_analysis.gradients_magnitude.len() {
            return false;
        }

        let transmissivity = groundwater_grid.transmissivities[idx];
        let conductivity = groundwater_grid.hydraulic_conductivities[idx];
        let gradient = gradient_analysis.gradients_magnitude[idx];

        // Criteria for aquifer suitability
        let min_transmissivity = 1e-6; // m²/s
        let min_conductivity = 1e-7;   // m/s
        let max_gradient = 10.0;       // Not too steep

        transmissivity > min_transmissivity &&
        conductivity > min_conductivity &&
        gradient < max_gradient
    }

    /// Generate springs from aquifer systems using Zig backend
    pub fn generate_springs_from_aquifers(
        &mut self,
        aquifers: &[Aquifer],
        elevation_data: &[f32],
        world_size: (u32, u32)
    ) -> Result<Vec<Spring>, SchedulerError> {
        let mut springs = Vec::new();
        let mut spring_id = 1u32;

        for aquifer in aquifers {
            let aquifer_springs = self.generate_springs_for_single_aquifer_zig(
                spring_id,
                aquifer,
                elevation_data,
                world_size
            )?;
            
            spring_id += aquifer_springs.len() as u32;
            springs.extend(aquifer_springs);
        }

        Ok(springs)
    }

    /// Generate springs for a single aquifer using Zig calculations
    fn generate_springs_for_single_aquifer_zig(
        &mut self,
        base_spring_id: u32,
        aquifer: &Aquifer,
        elevation_data: &[f32],
        world_size: (u32, u32),
    ) -> Result<Vec<Spring>, SchedulerError> {
        let mut springs = Vec::new();
        
        // Calculate number of springs based on aquifer properties
        let num_springs = self.calculate_spring_count_for_aquifer(aquifer);
        
        for i in 0..num_springs {
            if let Some(spring_location) = self.find_spring_location_zig(aquifer, elevation_data, world_size) {
                // Calculate spring properties using Zig backend
                let grid_pos = self.world_to_grid(spring_location);
                let surface_elevation = self.get_elevation_at_grid(grid_pos, elevation_data, world_size);
                
                // Use Zig backend to calculate discharge
                let hydraulic_head_difference = (aquifer.hydraulic_head - surface_elevation).max(0.0);
                let discharge = calculate_spring_discharge(
                    hydraulic_head_difference as f64,
                    aquifer.aquifer_type
                ) as f32;
                
                if discharge > 0.001 { // Minimum viable discharge
                    let spring_type = self.determine_spring_type_zig(
                        surface_elevation,
                        aquifer.hydraulic_head,
                        aquifer.aquifer_type
                    );
                    
                    let temperature = self.calculate_spring_temperature_zig(aquifer.depth);
                    let mineral_content = self.calculate_mineral_content_zig(aquifer);
                    
                    let spring = Spring {
                        id: base_spring_id + i,
                        position: spring_location,
                        flow_rate: discharge,
                        temperature,
                        aquifer_id: Some(aquifer.id),
                        mineral_content,
                        spring_type,
                    };
                    
                    springs.push(spring);
                }
            }
        }
        
        Ok(springs)
    }

    /// Calculate number of springs for an aquifer
    fn calculate_spring_count_for_aquifer(&self, aquifer: &Aquifer) -> u32 {
        let base_count = match aquifer.aquifer_type {
            AquiferType::Karst => 8,           // Many springs in karst
            AquiferType::FracturedRock => 6,   // Moderate number
            AquiferType::Unconfined => 4,      // Few springs
            AquiferType::Confined => 3,        // Fewer springs
            AquiferType::LeakyConfined => 3,   // Fewer springs
            AquiferType::Perched => 2,         // Very few
        };
        
        // Scale by aquifer extent
        let size_factor = (aquifer.extent / 1000.0).max(0.5).min(2.0); // 0.5x to 2x
        (base_count as f32 * size_factor) as u32
    }

    /// Determine spring type using Zig analysis
    fn determine_spring_type_zig(
        &self,
        surface_elevation: f32,
        hydraulic_head: f32,
        aquifer_type: AquiferType,
    ) -> SpringType {
        let head_difference = hydraulic_head - surface_elevation;
        
        match (aquifer_type, head_difference) {
            // High pressure springs
            (AquiferType::Confined, diff) if diff > 10.0 => SpringType::Artesian,
            (AquiferType::LeakyConfined, diff) if diff > 5.0 => SpringType::Artesian,
            
            // Contact springs at geological boundaries
            (AquiferType::FracturedRock, _) => SpringType::Contact,
            (AquiferType::Karst, _) => SpringType::Joint,
            
            // Depression springs in low areas
            (_, diff) if diff > 0.0 && surface_elevation < 50.0 => SpringType::Depression,
            
            // Default gravity springs
            _ => SpringType::Gravity,
        }
    }

    /// Calculate spring temperature using geothermal gradient
    fn calculate_spring_temperature_zig(&self, aquifer_depth: f32) -> f32 {
        let surface_temp = 12.0; // Average surface groundwater temperature (°C)
        let geothermal_gradient = 0.025; // 25°C per 1000m
        surface_temp + aquifer_depth * geothermal_gradient
    }

    /// Calculate mineral content using Zig backend analysis
    fn calculate_mineral_content_zig(&self, aquifer: &Aquifer) -> f32 {
        // Base mineral content on aquifer type and residence time
        let base_content = match aquifer.aquifer_type {
            AquiferType::Karst => 0.8,         // High mineralization
            AquiferType::FracturedRock => 0.6, // Moderate mineralization
            AquiferType::Confined => 0.7,      // High due to long residence time
            AquiferType::LeakyConfined => 0.5, // Moderate
            AquiferType::Unconfined => 0.3,    // Low
            AquiferType::Perched => 0.2,       // Very low
        };
        
        // Adjust for depth and recharge rate
        let depth_factor = (aquifer.depth / 100.0).min(1.0);
        let recharge_factor = (1.0 / (aquifer.recharge_rate + 0.1)).min(2.0);
        
        (base_content + depth_factor * 0.2 + recharge_factor * 0.1).min(1.0)
    }

    // Helper methods
    
    fn estimate_aquifer_thickness(&self, elevation: f32, aquifer_type: AquiferType) -> f32 {
        match aquifer_type {
            AquiferType::Unconfined => 20.0 + elevation * 0.1,
            AquiferType::Confined => 50.0 + elevation * 0.05,
            AquiferType::LeakyConfined => 30.0 + elevation * 0.08,
            AquiferType::Perched => 5.0 + elevation * 0.02,
            AquiferType::FracturedRock => 15.0 + elevation * 0.15,
            AquiferType::Karst => 40.0 + elevation * 0.12,
        }
    }

    fn calculate_porosity_for_type(&self, aquifer_type: AquiferType) -> f32 {
        match aquifer_type {
            AquiferType::Unconfined => 0.35,      // Sand and gravel
            AquiferType::Confined => 0.25,        // Sandstone
            AquiferType::LeakyConfined => 0.20,   // Mixed materials
            AquiferType::Perched => 0.30,         // Variable
            AquiferType::FracturedRock => 0.05,   // Low matrix porosity
            AquiferType::Karst => 0.15,           // Variable, caverns
        }
    }

    fn calculate_aquifer_depth_zig(&self, zone: &AquiferZone, _groundwater_grid: &GroundwaterGrid) -> f32 {
        match zone.dominant_type {
            AquiferType::Unconfined => self.config.aquifer_depth_range.0 + 20.0,
            AquiferType::Confined => self.config.aquifer_depth_range.1,
            AquiferType::LeakyConfined => (self.config.aquifer_depth_range.0 + self.config.aquifer_depth_range.1) / 2.0,
            AquiferType::Perched => self.config.aquifer_depth_range.0,
            AquiferType::FracturedRock => self.config.aquifer_depth_range.1 * 0.8,
            AquiferType::Karst => self.config.aquifer_depth_range.1 * 1.2,
        }
    }

    fn calculate_water_table_elevation_zig(&self, zone: &AquiferZone, _groundwater_grid: &GroundwaterGrid) -> f32 {
        // Simplified calculation - would use actual groundwater flow modeling
        zone.center.y as f32 * 0.1 + 10.0 // Basic topographic relationship
    }

    fn calculate_recharge_rate_zig(&self, zone: &AquiferZone) -> f32 {
        match zone.dominant_type {
            AquiferType::Unconfined => 0.15,      // 150mm/year
            AquiferType::Confined => 0.02,        // 20mm/year
            AquiferType::LeakyConfined => 0.08,   // 80mm/year
            AquiferType::Perched => 0.25,         // 250mm/year
            AquiferType::FracturedRock => 0.05,   // 50mm/year
            AquiferType::Karst => 0.30,           // 300mm/year
        }
    }

    fn generate_aquifer_boundary_zig(&self, zone: &AquiferZone) -> Vec<Vector2<f64>> {
        // Generate circular boundary for now - could be enhanced with irregular shapes
        let num_points = 16;
        let mut boundary = Vec::with_capacity(num_points);
        
        for i in 0..num_points {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / num_points as f64;
            let x = zone.center.x + zone.extent * angle.cos();
            let y = zone.center.y + zone.extent * angle.sin();
            boundary.push(Vector2::new(x, y));
        }
        
        boundary
    }

    fn find_spring_location_zig(
        &mut self,
        aquifer: &Aquifer,
        _elevation_data: &[f32],
        _world_size: (u32, u32),
    ) -> Option<Vector2<f64>> {
        // Simple random location on aquifer boundary for now
        if aquifer.boundary.is_empty() {
            return None;
        }
        
        let boundary_idx = self.rng.gen_range(0..aquifer.boundary.len());
        Some(aquifer.boundary[boundary_idx])
    }

    fn calculate_zone_extent(&self, potential_centers: &[(Vector2<f64>, usize)], cluster_indices: &[usize]) -> f64 {
        if cluster_indices.len() < 2 {
            return 100.0; // Default minimum extent
        }
        
        let positions: Vec<Vector2<f64>> = cluster_indices.iter()
            .filter_map(|&idx| potential_centers.get(idx).map(|(pos, _)| *pos))
            .collect();
        
        if positions.is_empty() {
            return 100.0;
        }
        
        // Calculate bounding box and use as extent estimate
        let min_x = positions.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max_x = positions.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = positions.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = positions.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        
        let width = max_x - min_x;
        let height = max_y - min_y;
        ((width * width + height * height).sqrt() / 2.0).max(100.0)
    }

    fn grid_to_world(&self, grid_x: usize, grid_y: usize) -> Vector2<f64> {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let x = min_x + (grid_x as f64 / self.config.grid_resolution as f64) * (max_x - min_x);
        let y = min_y + (grid_y as f64 / self.config.grid_resolution as f64) * (max_y - min_y);
        Vector2::new(x, y)
    }

    fn world_to_grid(&self, world_pos: Vector2<f64>) -> (usize, usize) {
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;
        let norm_x = ((world_pos.x - min_x) / (max_x - min_x)).clamp(0.0, 1.0);
        let norm_y = ((world_pos.y - min_y) / (max_y - min_y)).clamp(0.0, 1.0);
        
        let grid_x = (norm_x * (self.config.grid_resolution - 1) as f64) as usize;
        let grid_y = (norm_y * (self.config.grid_resolution - 1) as f64) as usize;
        
        (grid_x, grid_y)
    }

    fn get_elevation_at_grid(
        &self,
        grid_pos: (usize, usize),
        elevation_data: &[f32],
        world_size: (u32, u32)
    ) -> f32 {
        let (width, _height) = world_size;
        let (x, y) = grid_pos;
        let index = y * width as usize + x;
        
        if index < elevation_data.len() {
            elevation_data[index]
        } else {
            0.0
        }
    }
}

/// Intermediate structure for aquifer zone identification
#[derive(Debug, Clone)]
struct AquiferZone {
    center: Vector2<f64>,
    extent: f64,
    dominant_type: AquiferType,
    average_transmissivity: f64,
    average_conductivity: f64,
    cluster_size: usize,
}

/// Well pumping analysis using Zig Theis solution
pub fn analyze_well_pumping_effects(
    well_position: Vector2<f64>,
    pumping_rate: f64, // m³/s
    aquifer: &Aquifer,
    observation_points: &[Vector2<f64>],
    time_steps: &[f64], // seconds
) -> Vec<Vec<f64>> {
    let mut results = Vec::new();
    
    for &time in time_steps {
        let mut time_results = Vec::new();
        
        for &obs_point in observation_points {
            let distance = (obs_point - well_position).magnitude();
            let transmissivity = aquifer.permeability * aquifer.depth as f64;
            let storativity = match aquifer.aquifer_type {
                AquiferType::Unconfined => aquifer.porosity as f64 * 0.15, // Specific yield
                _ => 1e-4, // Specific storage * thickness
            };
            
            let drawdown = calculate_theis_solution(
                distance,
                time,
                pumping_rate,
                transmissivity,
                storativity,
            );
            
            time_results.push(drawdown);
        }
        
        results.push(time_results);
    }
    
    results
}
