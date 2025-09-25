//! Volcanic Activity System
//!
//! Generates volcanic zones, hot spots, and volcanic activity based on
//! tectonic plate interactions and mantle dynamics using statistical distributions.

use super::{TectonicsConfig, TectonicPlate, PlateBoundary, BoundaryType};
use serde::{Deserialize, Serialize};
use nalgebra::Vector2;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, Normal, Exp, Uniform, Poisson};
use rayon::prelude::*;
use crate::core::scheduler::SchedulerError;

/// Volcanic zone with activity characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolcanicZone {
    pub id: u32,
    pub name: String,
    pub zone_type: VolcanicZoneType,
    pub center: Vector2<f64>,
    pub radius: f64,
    pub volcanoes: Vec<Volcano>,
    pub activity_level: ActivityLevel,
    pub magma_composition: MagmaComposition,
    pub associated_boundary_id: Option<u32>,
    pub hotspot_id: Option<u32>,
}

/// Individual volcano within a zone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volcano {
    pub id: u32,
    pub position: Vector2<f64>,
    pub elevation: f64,
    pub volcano_type: VolcanoType,
    pub activity_level: ActivityLevel,
    pub last_eruption_years_ago: f64,
    pub vei_scale: u32, // Volcanic Explosivity Index (0-8)
    pub magma_chamber_depth: f64,
    pub hazard_radius: f64,
}

/// Types of volcanic zones
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VolcanicZoneType {
    SubductionZone,  // Oceanic plate subducting
    RiftZone,        // Divergent boundary volcanism
    IslandArc,       // Oceanic-oceanic subduction
    ContinentalArc,  // Oceanic-continental subduction
    Hotspot,         // Mantle plume hotspot
    BackArc,         // Behind subduction zones
}

/// Types of individual volcanoes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VolcanoType {
    Stratovolcano,   // Composite volcano
    Shield,          // Shield volcano
    Cinder,          // Cinder cone
    Caldera,         // Large caldera system
    Fissure,         // Fissure eruption
    Submarine,       // Underwater volcano
}

/// Volcanic activity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ActivityLevel {
    Dormant,    // No recent activity
    Low,        // Minimal activity
    Moderate,   // Regular activity
    High,       // Frequent activity
    Extreme,    // Constant activity
}

/// Magma composition types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MagmaComposition {
    Basaltic,    // Low silica, high temperature
    Andesitic,   // Intermediate silica
    Dacitic,     // High silica
    Rhyolitic,   // Very high silica, explosive
}

/// Volcanic system generator
#[derive(Debug, Clone)]
pub struct VolcanicSystem {
    config: TectonicsConfig,
    rng: ChaCha8Rng,
}

impl VolcanicSystem {
    /// Create new volcanic system
    pub fn new(config: &TectonicsConfig) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(config.seed + 3);
        Self {
            config: config.clone(),
            rng,
        }
    }

    /// Generate volcanic zones from plates and boundaries
    pub fn generate_volcanic_zones(
        &self,
        plates: &[TectonicPlate],
        boundaries: &[PlateBoundary],
    ) -> Result<Vec<VolcanicZone>, SchedulerError> {
        let mut zones = Vec::new();

        // Generate boundary-related volcanic zones in parallel
        let boundary_zones: Result<Vec<_>, _> = boundaries
            .par_iter()
            .enumerate()
            .filter_map(|(id, boundary)| {
                match self.create_volcanic_zone_from_boundary(id as u32, boundary, plates) {
                    Ok(Some(zone)) => Some(Ok(zone)),
                    Ok(None) => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .collect();

        zones.extend(boundary_zones.map_err(|e| SchedulerError::TaskFailed(e.to_string()))?);

        // Generate hotspot volcanic zones
        let hotspot_zones = self.generate_hotspot_zones(plates)?;
        zones.extend(hotspot_zones);

        Ok(zones)
    }

    /// Create volcanic zone from plate boundary
    fn create_volcanic_zone_from_boundary(
        &self,
        id: u32,
        boundary: &PlateBoundary,
        plates: &[TectonicPlate],
    ) -> Result<Option<VolcanicZone>, String> {
        let zone_type = match boundary.boundary_type {
            BoundaryType::Convergent => {
                // Determine if this creates volcanism
                let plate1 = plates.iter().find(|p| p.id == boundary.plate1_id)
                    .ok_or("Plate1 not found")?;
                let plate2 = plates.iter().find(|p| p.id == boundary.plate2_id)
                    .ok_or("Plate2 not found")?;

                self.determine_convergent_volcanic_type(plate1, plate2)
            }
            BoundaryType::Divergent => {
                // Divergent boundaries create rift volcanism
                Some(VolcanicZoneType::RiftZone)
            }
            BoundaryType::Transform => {
                // Transform boundaries rarely create volcanism
                return Ok(None);
            }
        };

        let zone_type = match zone_type {
            Some(t) => t,
            None => return Ok(None), // No volcanism for this boundary
        };

        let mut rng = self.rng.clone();

        // Calculate zone properties
        let center = self.calculate_zone_center(&boundary.geometry);
        let radius = self.calculate_zone_radius(zone_type, boundary.length);
        let activity_level = self.determine_activity_level(zone_type, boundary, &mut rng);
        let magma_composition = self.determine_magma_composition(zone_type, &mut rng);

        // Generate volcanoes within the zone
        let volcanoes = self.generate_volcanoes_in_zone(
            &center,
            radius,
            zone_type,
            activity_level,
            magma_composition,
            &mut rng,
        )?;

        Ok(Some(VolcanicZone {
            id,
            name: format!("Volcanic_Zone_{}", id),
            zone_type,
            center,
            radius,
            volcanoes,
            activity_level,
            magma_composition,
            associated_boundary_id: Some(boundary.id),
            hotspot_id: None,
        }))
    }

    /// Determine volcanic zone type for convergent boundaries
    fn determine_convergent_volcanic_type(
        &self,
        plate1: &TectonicPlate,
        plate2: &TectonicPlate,
    ) -> Option<VolcanicZoneType> {
        use super::PlateType;

        match (plate1.plate_type, plate2.plate_type) {
            (PlateType::Oceanic, PlateType::Continental) | (PlateType::Continental, PlateType::Oceanic) => {
                Some(VolcanicZoneType::ContinentalArc)
            }
            (PlateType::Oceanic, PlateType::Oceanic) => {
                Some(VolcanicZoneType::IslandArc)
            }
            (PlateType::Continental, PlateType::Continental) => {
                // Continental collision rarely produces volcanism
                None
            }
            _ => {
                // Mixed plates - 50% chance of volcanism
                if rand::random() {
                    Some(VolcanicZoneType::SubductionZone)
                } else {
                    None
                }
            }
        }
    }

    /// Calculate center of volcanic zone from boundary geometry
    fn calculate_zone_center(&self, geometry: &[Vector2<f64>]) -> Vector2<f64> {
        if geometry.is_empty() {
            return Vector2::new(0.0, 0.0);
        }

        let sum = geometry.iter().fold(Vector2::new(0.0, 0.0), |acc, p| acc + p);
        sum / geometry.len() as f64
    }

    /// Calculate zone radius based on type and boundary length
    fn calculate_zone_radius(&self, zone_type: VolcanicZoneType, boundary_length: f64) -> f64 {
        let base_radius = match zone_type {
            VolcanicZoneType::SubductionZone => boundary_length * 0.3,
            VolcanicZoneType::RiftZone => boundary_length * 0.2,
            VolcanicZoneType::IslandArc => boundary_length * 0.4,
            VolcanicZoneType::ContinentalArc => boundary_length * 0.5,
            VolcanicZoneType::Hotspot => 200.0, // Fixed radius for hotspots
            VolcanicZoneType::BackArc => boundary_length * 0.25,
        };

        base_radius.max(50.0).min(500.0) // 50-500 km radius
    }

    /// Determine activity level based on zone type and boundary properties
    fn determine_activity_level(
        &self,
        zone_type: VolcanicZoneType,
        boundary: &PlateBoundary,
        rng: &mut ChaCha8Rng,
    ) -> ActivityLevel {
        let base_activity = match zone_type {
            VolcanicZoneType::SubductionZone => ActivityLevel::High,
            VolcanicZoneType::RiftZone => ActivityLevel::Moderate,
            VolcanicZoneType::IslandArc => ActivityLevel::High,
            VolcanicZoneType::ContinentalArc => ActivityLevel::High,
            VolcanicZoneType::Hotspot => ActivityLevel::Moderate,
            VolcanicZoneType::BackArc => ActivityLevel::Low,
        };

        // Modify based on boundary properties
        let velocity_factor = (boundary.relative_velocity * 100.0).min(2.0);
        let stress_factor = (boundary.stress_magnitude / 1e6).min(2.0);
        
        let activity_modifier = (velocity_factor + stress_factor) * self.config.volcanic_intensity;

        // Apply randomness and modifiers
        let random_factor = rng.gen_range(0.8..1.2);
        let final_activity = activity_modifier * random_factor;

        match final_activity {
            x if x < 1.0 => ActivityLevel::Dormant,
            x if x < 2.0 => ActivityLevel::Low,
            x if x < 3.0 => ActivityLevel::Moderate,
            x if x < 4.0 => ActivityLevel::High,
            _ => ActivityLevel::Extreme,
        }
    }

    /// Determine magma composition for zone type
    fn determine_magma_composition(&self, zone_type: VolcanicZoneType, rng: &mut ChaCha8Rng) -> MagmaComposition {
        let compositions = match zone_type {
            VolcanicZoneType::RiftZone | VolcanicZoneType::Hotspot => {
                // Basaltic compositions dominate
                vec![
                    (MagmaComposition::Basaltic, 0.8),
                    (MagmaComposition::Andesitic, 0.2),
                ]
            }
            VolcanicZoneType::IslandArc => {
                // Mixed but favoring andesitic
                vec![
                    (MagmaComposition::Basaltic, 0.3),
                    (MagmaComposition::Andesitic, 0.5),
                    (MagmaComposition::Dacitic, 0.2),
                ]
            }
            VolcanicZoneType::ContinentalArc | VolcanicZoneType::SubductionZone => {
                // More evolved compositions
                vec![
                    (MagmaComposition::Andesitic, 0.4),
                    (MagmaComposition::Dacitic, 0.4),
                    (MagmaComposition::Rhyolitic, 0.2),
                ]
            }
            VolcanicZoneType::BackArc => {
                // Variable composition
                vec![
                    (MagmaComposition::Basaltic, 0.4),
                    (MagmaComposition::Andesitic, 0.4),
                    (MagmaComposition::Dacitic, 0.2),
                ]
            }
        };

        // Weighted random selection
        let random_value: f64 = rng.gen();
        let mut cumulative = 0.0;

        for (composition, weight) in compositions {
            cumulative += weight;
            if random_value <= cumulative {
                return composition;
            }
        }

        MagmaComposition::Andesitic // Fallback
    }

    /// Generate individual volcanoes within a zone
    fn generate_volcanoes_in_zone(
        &self,
        center: &Vector2<f64>,
        radius: f64,
        zone_type: VolcanicZoneType,
        activity_level: ActivityLevel,
        magma_composition: MagmaComposition,
        rng: &mut ChaCha8Rng,
    ) -> Result<Vec<Volcano>, String> {
        let mut volcanoes = Vec::new();

        // Determine number of volcanoes based on zone type and activity
        let base_count = match zone_type {
            VolcanicZoneType::SubductionZone => 8,
            VolcanicZoneType::RiftZone => 12,
            VolcanicZoneType::IslandArc => 15,
            VolcanicZoneType::ContinentalArc => 10,
            VolcanicZoneType::Hotspot => 6,
            VolcanicZoneType::BackArc => 5,
        };

        let activity_multiplier = match activity_level {
            ActivityLevel::Dormant => 0.3,
            ActivityLevel::Low => 0.6,
            ActivityLevel::Moderate => 1.0,
            ActivityLevel::High => 1.5,
            ActivityLevel::Extreme => 2.0,
        };

        let volcano_count = (base_count as f64 * activity_multiplier * self.config.volcanic_intensity) as u32;
        let volcano_count = volcano_count.max(1).min(50);

        // Generate volcanoes with Poisson distribution for realistic clustering
        let poisson = Poisson::new(volcano_count as f64).map_err(|e| e.to_string())?;
        let actual_count = poisson.sample(rng) as u32;

        for i in 0..actual_count {
            let volcano = self.generate_individual_volcano(
                i,
                center,
                radius,
                zone_type,
                activity_level,
                magma_composition,
                rng,
            )?;
            volcanoes.push(volcano);
        }

        Ok(volcanoes)
    }

    /// Generate individual volcano properties
    fn generate_individual_volcano(
        &self,
        id: u32,
        zone_center: &Vector2<f64>,
        zone_radius: f64,
        zone_type: VolcanicZoneType,
        zone_activity: ActivityLevel,
        magma_composition: MagmaComposition,
        rng: &mut ChaCha8Rng,
    ) -> Result<Volcano, String> {
        // Generate position within zone (clustered toward center)
        let distance_from_center = rng.gen_range(0.0..zone_radius) * rng.gen_range(0.3..1.0);
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        let position = zone_center + Vector2::new(
            distance_from_center * angle.cos(),
            distance_from_center * angle.sin(),
        );

        // Determine volcano type based on zone type and magma composition
        let volcano_type = self.determine_volcano_type(zone_type, magma_composition, rng);

        // Generate elevation based on volcano type
        let elevation = self.generate_volcano_elevation(volcano_type, rng);

        // Individual activity level (can vary from zone average)
        let activity_level = self.vary_individual_activity(zone_activity, rng);

        // Generate eruption history
        let last_eruption_years_ago = self.generate_last_eruption_time(activity_level, rng);

        // Volcanic Explosivity Index based on magma composition
        let vei_scale = self.determine_vei_scale(magma_composition, volcano_type, rng);

        // Magma chamber depth
        let magma_chamber_depth = self.generate_chamber_depth(volcano_type, rng);

        // Hazard radius based on VEI and volcano type
        let hazard_radius = self.calculate_hazard_radius(vei_scale, volcano_type);

        Ok(Volcano {
            id,
            position,
            elevation,
            volcano_type,
            activity_level,
            last_eruption_years_ago,
            vei_scale,
            magma_chamber_depth,
            hazard_radius,
        })
    }

    /// Determine individual volcano type
    fn determine_volcano_type(
        &self,
        zone_type: VolcanicZoneType,
        magma_composition: MagmaComposition,
        rng: &mut ChaCha8Rng,
    ) -> VolcanoType {
        let probabilities = match (zone_type, magma_composition) {
            (VolcanicZoneType::RiftZone, MagmaComposition::Basaltic) => {
                vec![(VolcanoType::Shield, 0.4), (VolcanoType::Fissure, 0.4), (VolcanoType::Cinder, 0.2)]
            }
            (VolcanicZoneType::Hotspot, _) => {
                vec![(VolcanoType::Shield, 0.6), (VolcanoType::Cinder, 0.3), (VolcanoType::Caldera, 0.1)]
            }
            (_, MagmaComposition::Rhyolitic) => {
                vec![(VolcanoType::Caldera, 0.4), (VolcanoType::Stratovolcano, 0.6)]
            }
            _ => {
                vec![(VolcanoType::Stratovolcano, 0.5), (VolcanoType::Cinder, 0.3), (VolcanoType::Shield, 0.2)]
            }
        };

        // Weighted selection
        let random_value: f64 = rng.gen();
        let mut cumulative = 0.0;

        for (volcano_type, weight) in probabilities {
            cumulative += weight;
            if random_value <= cumulative {
                return volcano_type;
            }
        }

        VolcanoType::Stratovolcano // Fallback
    }

    /// Generate volcano elevation
    fn generate_volcano_elevation(&self, volcano_type: VolcanoType, rng: &mut ChaCha8Rng) -> f64 {
        let (mean, std_dev) = match volcano_type {
            VolcanoType::Stratovolcano => (3000.0, 1000.0),
            VolcanoType::Shield => (1500.0, 500.0),
            VolcanoType::Cinder => (800.0, 300.0),
            VolcanoType::Caldera => (2000.0, 800.0),
            VolcanoType::Fissure => (500.0, 200.0),
            VolcanoType::Submarine => (-1000.0, 500.0),
        };

        let normal = Normal::new(mean, std_dev).unwrap();
        (normal.sample(rng) as f64).max(-2000.0).min(8000.0)
    }

    /// Vary individual volcano activity from zone average
    fn vary_individual_activity(&self, zone_activity: ActivityLevel, rng: &mut ChaCha8Rng) -> ActivityLevel {
        let zone_value = zone_activity as u8;
        let variation = rng.gen_range(-1..=1);
        let new_value = ((zone_value as i8) + variation).clamp(0, 4) as u8;

        match new_value {
            0 => ActivityLevel::Dormant,
            1 => ActivityLevel::Low,
            2 => ActivityLevel::Moderate,
            3 => ActivityLevel::High,
            4 => ActivityLevel::Extreme,
            _ => zone_activity,
        }
    }

    /// Generate time since last eruption
    fn generate_last_eruption_time(&self, activity_level: ActivityLevel, rng: &mut ChaCha8Rng) -> f64 {
        let lambda = match activity_level {
            ActivityLevel::Dormant => 0.0001,   // Very rare eruptions
            ActivityLevel::Low => 0.001,       // Every ~1000 years
            ActivityLevel::Moderate => 0.01,   // Every ~100 years
            ActivityLevel::High => 0.1,        // Every ~10 years
            ActivityLevel::Extreme => 1.0,     // Every year
        };

        if lambda <= 0.0001 {
            return rng.gen_range(10000.0..100000.0); // Very long dormancy
        }

        let exponential = Exp::new(lambda).unwrap();
        (exponential.sample(rng) as f64).min(100000.0)
    }

    /// Determine VEI scale for volcano
    fn determine_vei_scale(&self, magma_composition: MagmaComposition, volcano_type: VolcanoType, rng: &mut ChaCha8Rng) -> u32 {
        let base_vei = match magma_composition {
            MagmaComposition::Basaltic => 2,     // Generally gentle
            MagmaComposition::Andesitic => 3,    // Moderate explosivity  
            MagmaComposition::Dacitic => 4,      // High explosivity
            MagmaComposition::Rhyolitic => 5,    // Very high explosivity
        };

        let type_modifier = match volcano_type {
            VolcanoType::Shield | VolcanoType::Fissure => -1,
            VolcanoType::Cinder => 0,
            VolcanoType::Stratovolcano => 0,
            VolcanoType::Caldera => 2,
            VolcanoType::Submarine => -1,
        };

        let random_variation = rng.gen_range(-1..=1);
        ((base_vei as i32) + type_modifier + random_variation).clamp(0, 8) as u32
    }

    /// Generate magma chamber depth
    fn generate_chamber_depth(&self, volcano_type: VolcanoType, rng: &mut ChaCha8Rng) -> f64 {
        let (mean_depth, variation) = match volcano_type {
            VolcanoType::Shield => (15.0, 5.0),        // Shallow
            VolcanoType::Fissure => (5.0, 2.0),        // Very shallow
            VolcanoType::Cinder => (10.0, 3.0),        // Shallow
            VolcanoType::Stratovolcano => (8.0, 4.0),   // Medium
            VolcanoType::Caldera => (25.0, 10.0),      // Deep
            VolcanoType::Submarine => (12.0, 6.0),     // Variable
        };

        let uniform = Uniform::new(mean_depth - variation, mean_depth + variation);
        (uniform.sample(rng) as f64).max(1.0)
    }

    /// Calculate hazard radius
    fn calculate_hazard_radius(&self, vei_scale: u32, volcano_type: VolcanoType) -> f64 {
        let base_radius = match vei_scale {
            0..=1 => 5.0,
            2 => 10.0,
            3 => 25.0,
            4 => 50.0,
            5 => 100.0,
            6 => 200.0,
            7 => 500.0,
            8 => 1000.0,
            _ => 100.0,
        };

        let type_multiplier = match volcano_type {
            VolcanoType::Caldera => 2.0,
            VolcanoType::Stratovolcano => 1.5,
            VolcanoType::Shield => 0.8,
            VolcanoType::Fissure => 0.6,
            VolcanoType::Cinder => 0.5,
            VolcanoType::Submarine => 0.3,
        };

        base_radius * type_multiplier
    }

    /// Generate hotspot volcanic zones
    fn generate_hotspot_zones(&self, plates: &[TectonicPlate]) -> Result<Vec<VolcanicZone>, SchedulerError> {
        let mut rng = self.rng.clone();
        let mut hotspot_zones = Vec::new();

        // Generate 3-8 hotspots randomly distributed
        let hotspot_count = rng.gen_range(3..=8);
        let (min_x, min_y, max_x, max_y) = self.config.world_bounds;

        for i in 0..hotspot_count {
            let center = Vector2::new(
                rng.gen_range(min_x..max_x),
                rng.gen_range(min_y..max_y),
            );

            let radius = rng.gen_range(100.0..300.0);
            let activity_level = match rng.gen_range(0..4) {
                0 => ActivityLevel::Low,
                1 => ActivityLevel::Moderate,
                2 => ActivityLevel::High,
                _ => ActivityLevel::Extreme,
            };

            let volcanoes = self.generate_volcanoes_in_zone(
                &center,
                radius,
                VolcanicZoneType::Hotspot,
                activity_level,
                MagmaComposition::Basaltic,
                &mut rng,
            ).map_err(|e| SchedulerError::TaskFailed(e.to_string()))?;

            hotspot_zones.push(VolcanicZone {
                id: 1000 + i, // Offset hotspot IDs
                name: format!("Hotspot_{}", i),
                zone_type: VolcanicZoneType::Hotspot,
                center,
                radius,
                volcanoes,
                activity_level,
                magma_composition: MagmaComposition::Basaltic,
                associated_boundary_id: None,
                hotspot_id: Some(i),
            });
        }

        Ok(hotspot_zones)
    }
}
