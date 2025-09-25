//! Configuration for edge detection algorithms

/// Edge detection algorithms and parameters
#[derive(Debug, Clone)]
pub struct EdgeDetectionConfig {
    /// Sobel operator kernels for edge detection
    pub sobel_threshold: f32,
    /// Canny edge detection parameters
    pub canny_low_threshold: f32,
    pub canny_high_threshold: f32,
    /// Gaussian blur parameters for noise reduction
    pub gaussian_blur_sigma: f32,
    /// Minimum edge length to consider significant
    pub min_edge_length: u32,
    /// Maximum gap size to bridge in edge linking
    pub max_gap_size: u32,
}

impl Default for EdgeDetectionConfig {
    fn default() -> Self {
        Self {
            sobel_threshold: 0.1,
            canny_low_threshold: 0.05,
            canny_high_threshold: 0.15,
            gaussian_blur_sigma: 0.8,
            min_edge_length: 3,
            max_gap_size: 2,
        }
    }
}
