//! LOD (Level of Detail) system utilities
//! Manages distance-based detail levels for optimal performance

use crate::core::zig_ffi::HexCoord;

/// LOD distance thresholds in hex units
pub const LOD_THRESHOLDS: LODThresholds = LODThresholds {
    // Full geometry, textures, resources  
    high_detail: 15,
    // Simplified geometry, basic colors
    medium_detail: 35,
    // Single triangles, biome colors only
    low_detail: 70,
    // Not rendered
    culled: 100,
};

/// LOD level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LODLevel {
    High = 0,
    Medium = 1,
    Low = 2,
    Culled = 3,
}

/// LOD distance thresholds
pub struct LODThresholds {
    pub high_detail: u32,
    pub medium_detail: u32,
    pub low_detail: u32,
    pub culled: u32,
}

/// Calculate LOD level based on hex distance from camera with zoom scaling
pub fn calculate_lod_level(camera_hex: HexCoord, tile_hex: HexCoord) -> LODLevel {
    let distance = crate::core::zig_ffi::hex_distance(camera_hex, tile_hex);
    
    if distance <= LOD_THRESHOLDS.high_detail {
        LODLevel::High
    } else if distance <= LOD_THRESHOLDS.medium_detail {
        LODLevel::Medium
    } else if distance <= LOD_THRESHOLDS.low_detail {
        LODLevel::Low
    } else {
        LODLevel::Culled
    }
}

/// Calculate LOD level with zoom scaling support
pub fn calculate_lod_level_with_zoom(camera_hex: HexCoord, tile_hex: HexCoord, zoom_level: f32) -> LODLevel {
    let distance = crate::core::zig_ffi::hex_distance(camera_hex, tile_hex);
    
    // Scale LOD thresholds based on zoom level
    // When zoomed out (zoom < 1), increase thresholds to show more tiles
    // When zoomed in (zoom > 1), decrease thresholds for better detail
    let zoom_scale = (1.0 / zoom_level).max(0.1);
    
    let scaled_thresholds = LODThresholds {
        high_detail: ((LOD_THRESHOLDS.high_detail as f32) * zoom_scale) as u32,
        medium_detail: ((LOD_THRESHOLDS.medium_detail as f32) * zoom_scale) as u32,
        low_detail: ((LOD_THRESHOLDS.low_detail as f32) * zoom_scale) as u32,
        culled: ((LOD_THRESHOLDS.culled as f32) * zoom_scale) as u32,
    };
    
    if distance <= scaled_thresholds.high_detail {
        LODLevel::High
    } else if distance <= scaled_thresholds.medium_detail {
        LODLevel::Medium
    } else if distance <= scaled_thresholds.low_detail {
        LODLevel::Low
    } else {
        LODLevel::Culled
    }
}

/// Check if tile should be rendered at given LOD level
pub fn should_render_at_lod(
    camera_hex: HexCoord,
    tile_hex: HexCoord,
    requested_lods: &[u8],
) -> bool {
    let tile_lod = calculate_lod_level(camera_hex, tile_hex) as u8;
    requested_lods.contains(&tile_lod)
}

/// Check if tile should be rendered at given LOD level with zoom scaling
pub fn should_render_at_lod_with_zoom(
    camera_hex: HexCoord,
    tile_hex: HexCoord,
    requested_lods: &[u8],
    zoom_level: f32,
) -> bool {
    let tile_lod = calculate_lod_level_with_zoom(camera_hex, tile_hex, zoom_level) as u8;
    requested_lods.contains(&tile_lod)
}

/// Batch calculate LOD levels for multiple tiles
pub fn calculate_lod_levels(
    camera_hex: HexCoord,
    tile_hexes: &[HexCoord],
) -> Vec<LODLevel> {
    tile_hexes
        .iter()
        .map(|&tile_hex| calculate_lod_level(camera_hex, tile_hex))
        .collect()
}

/// Get maximum render distance for quality level
pub fn get_max_render_distance(quality: &str) -> u32 {
    match quality.to_lowercase().as_str() {
        "low" => LOD_THRESHOLDS.low_detail,
        "medium" => LOD_THRESHOLDS.low_detail,
        "high" => LOD_THRESHOLDS.low_detail,
        _ => LOD_THRESHOLDS.low_detail,
    }
}

impl LODLevel {
    /// Convert to float for GPU instance data
    pub fn to_f32(self) -> f32 {
        self as u8 as f32
    }
    
    /// Get level display name
    pub fn name(self) -> &'static str {
        match self {
            LODLevel::High => "High Detail",
            LODLevel::Medium => "Medium Detail", 
            LODLevel::Low => "Low Detail",
            LODLevel::Culled => "Culled",
        }
    }
}

impl From<u8> for LODLevel {
    fn from(value: u8) -> Self {
        match value {
            0 => LODLevel::High,
            1 => LODLevel::Medium,
            2 => LODLevel::Low,
            _ => LODLevel::Culled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_calculation() {
        let camera = HexCoord { q: 0, r: 0 };
        
        // Test high detail range (distance <= 15)
        let close_tile = HexCoord { q: 5, r: 5 }; // Distance: ~8.6
        assert_eq!(calculate_lod_level(camera, close_tile), LODLevel::High);
        
        // Test edge of high detail range
        let edge_high_tile = HexCoord { q: 10, r: 5 }; // Distance: 15
        assert_eq!(calculate_lod_level(camera, edge_high_tile), LODLevel::High);
        
        // Test medium detail range (distance <= 35)
        let medium_tile = HexCoord { q: 20, r: 10 }; // Distance: ~26.5
        assert_eq!(calculate_lod_level(camera, medium_tile), LODLevel::Medium);
        
        // Test low detail range (distance <= 70)
        let far_tile = HexCoord { q: 40, r: 20 }; // Distance: ~52
        assert_eq!(calculate_lod_level(camera, far_tile), LODLevel::Low);
        
        // Test culled range (distance > 70)
        let very_far_tile = HexCoord { q: 80, r: 40 }; // Distance: ~104
        assert_eq!(calculate_lod_level(camera, very_far_tile), LODLevel::Culled);
    }
    
    #[test]
    fn test_should_render_filtering() {
        let camera = HexCoord { q: 0, r: 0 };
        let close_tile = HexCoord { q: 5, r: 5 }; // High detail
        let medium_tile = HexCoord { q: 20, r: 10 }; // Medium detail
        let far_tile = HexCoord { q: 50, r: 25 }; // Low detail
        let very_far_tile = HexCoord { q: 80, r: 40 }; // Culled
        
        // Should render with high detail LOD
        assert!(should_render_at_lod(camera, close_tile, &[0]));
        assert!(!should_render_at_lod(camera, close_tile, &[2]));
        
        // Medium detail tile
        assert!(!should_render_at_lod(camera, medium_tile, &[0])); // Not high detail
        assert!(should_render_at_lod(camera, medium_tile, &[1])); // Is medium detail
        
        // Low detail tile
        assert!(should_render_at_lod(camera, far_tile, &[2])); // Is low detail
        assert!(!should_render_at_lod(camera, far_tile, &[0, 1])); // Not high/medium
        
        // Culled tile should never render
        assert!(!should_render_at_lod(camera, very_far_tile, &[0, 1, 2]));
        
        // Should render with multiple LODs including appropriate one
        assert!(should_render_at_lod(camera, close_tile, &[0, 1, 2]));
        assert!(should_render_at_lod(camera, medium_tile, &[1, 2]));
    }
    
    #[test]
    fn test_batch_lod_calculation() {
        let camera = HexCoord { q: 0, r: 0 };
        let tiles = vec![
            HexCoord { q: 5, r: 5 },    // High
            HexCoord { q: 20, r: 10 },  // Medium
            HexCoord { q: 50, r: 25 },  // Low
            HexCoord { q: 80, r: 40 },  // Culled
        ];
        
        let lod_levels = calculate_lod_levels(camera, &tiles);
        
        assert_eq!(lod_levels[0], LODLevel::High);
        assert_eq!(lod_levels[1], LODLevel::Medium);
        assert_eq!(lod_levels[2], LODLevel::Low);
        assert_eq!(lod_levels[3], LODLevel::Culled);
    }
    
    #[test]
    fn test_lod_level_conversions() {
        // Test to_f32
        assert_eq!(LODLevel::High.to_f32(), 0.0);
        assert_eq!(LODLevel::Medium.to_f32(), 1.0);
        assert_eq!(LODLevel::Low.to_f32(), 2.0);
        assert_eq!(LODLevel::Culled.to_f32(), 3.0);
        
        // Test from u8
        assert_eq!(LODLevel::from(0), LODLevel::High);
        assert_eq!(LODLevel::from(1), LODLevel::Medium);
        assert_eq!(LODLevel::from(2), LODLevel::Low);
        assert_eq!(LODLevel::from(3), LODLevel::Culled);
        assert_eq!(LODLevel::from(99), LODLevel::Culled); // Out of range = culled
        
        // Test name
        assert_eq!(LODLevel::High.name(), "High Detail");
        assert_eq!(LODLevel::Medium.name(), "Medium Detail");
        assert_eq!(LODLevel::Low.name(), "Low Detail");
        assert_eq!(LODLevel::Culled.name(), "Culled");
    }
    
    #[test]
    fn test_distance_edge_cases() {
        let camera = HexCoord { q: 0, r: 0 };
        
        // Test same position (distance = 0)
        assert_eq!(calculate_lod_level(camera, camera), LODLevel::High);
        
        // Test exact threshold boundaries
        let exactly_15_away = HexCoord { q: 15, r: 0 }; // Distance exactly 15
        assert_eq!(calculate_lod_level(camera, exactly_15_away), LODLevel::High);
        
        let just_over_15 = HexCoord { q: 16, r: 0 }; // Distance 16
        assert_eq!(calculate_lod_level(camera, just_over_15), LODLevel::Medium);
        
        // Test negative coordinates
        let negative_tile = HexCoord { q: -10, r: -5 };
        let lod = calculate_lod_level(camera, negative_tile);
        assert!(matches!(lod, LODLevel::High | LODLevel::Medium)); // Should work with negatives
    }
    
    #[test]
    fn test_quality_distance_mapping() {
        assert_eq!(get_max_render_distance("low"), LOD_THRESHOLDS.low_detail);
        assert_eq!(get_max_render_distance("medium"), LOD_THRESHOLDS.low_detail);
        assert_eq!(get_max_render_distance("high"), LOD_THRESHOLDS.low_detail);
        assert_eq!(get_max_render_distance("invalid"), LOD_THRESHOLDS.low_detail);
    }
    
    #[test]
    fn test_lod_thresholds_are_sensible() {
        // Ensure thresholds are in ascending order
        assert!(LOD_THRESHOLDS.high_detail < LOD_THRESHOLDS.medium_detail);
        assert!(LOD_THRESHOLDS.medium_detail < LOD_THRESHOLDS.low_detail);
        assert!(LOD_THRESHOLDS.low_detail < LOD_THRESHOLDS.culled);
        
        // Ensure they're reasonable values
        assert!(LOD_THRESHOLDS.high_detail > 0);
        assert!(LOD_THRESHOLDS.culled <= 200); // Not too large
    }
}
