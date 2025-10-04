//! High-performance noise caching system
//!
//! Integrates with the existing GameCache system to provide
//! efficient caching of noise calculations with spatial coherence.

use crate::core::caching::{CacheKeyTrait, CacheStats};
use super::types::*;
use super::NoiseResult;

use dashmap::DashMap;
use moka::sync::{Cache, CacheBuilder};
use quick_cache::sync::Cache as QuickCache;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Specialized noise cache key for spatial coherence
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoiseCacheKey {
    /// Spatial coordinates (quantized for better cache hits)
    pub x: i64,
    pub y: i64,
    /// Noise type identifier
    pub noise_type: NoiseType,
    /// Configuration hash for cache invalidation
    pub config_hash: u64,
    /// Zoom level for LOD caching
    pub zoom_level: u32,
}

impl NoiseCacheKey {
    /// Create cache key with spatial quantization
    pub fn new(x: f64, y: f64, noise_type: NoiseType, config_hash: u64, zoom_level: u32) -> Self {
        // Quantize coordinates for better cache coherence
        let grid_size = match zoom_level {
            0..=2 => 1.0,      // High detail
            3..=5 => 0.5,      // Medium detail  
            6..=8 => 0.25,     // Low detail
            _ => 0.125,        // Very low detail
        };

        Self {
            x: (x / grid_size).round() as i64,
            y: (y / grid_size).round() as i64,
            noise_type,
            config_hash,
            zoom_level,
        }
    }

    /// Create hash from coordinates and type for fast lookups
    pub fn fast_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl CacheKeyTrait for NoiseCacheKey {
    fn size_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn priority(&self) -> u32 {
        // Higher priority for lower zoom levels (more detailed)
        match self.zoom_level {
            0..=2 => 100,
            3..=5 => 75,
            6..=8 => 50,
            _ => 25,
        }
    }
}

/// Cached noise value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedNoiseValue {
    /// The noise result
    pub result: NoiseResult,
    /// Timestamp for cache validation (as seconds since UNIX_EPOCH)
    pub timestamp_secs: u64,
    /// Access count for LRU eviction
    pub access_count: u32,
    /// Spatial coherence score
    pub coherence_score: f32,
}

impl CachedNoiseValue {
    /// Create new cached value with current timestamp
    pub fn new(result: NoiseResult, coherence_score: f32) -> Self {
        let timestamp_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Self {
            result,
            timestamp_secs,
            access_count: 1,
            coherence_score,
        }
    }
    
    /// Check if value is still valid (not expired)
    pub fn is_valid(&self, max_age_secs: u64) -> bool {
        let current_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        current_secs - self.timestamp_secs <= max_age_secs
    }
    
    /// Mark as accessed
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
    }
}

/// High-performance noise cache with multiple backend strategies
#[derive(Debug)]
pub struct NoiseCache {
    /// Primary cache using moka for complex eviction policies
    primary: Cache<NoiseCacheKey, CachedNoiseValue>,
    /// Quick cache for hot paths
    quick: Arc<QuickCache<u64, f32>>,
    /// Spatial index for coherent access patterns
    spatial: DashMap<(i64, i64), Vec<NoiseCacheKey>>,
    /// Cache statistics
    stats: Arc<std::sync::Mutex<CacheStats>>,
    /// Configuration
    config: NoiseCacheConfig,
}

/// Noise cache configuration
#[derive(Debug, Clone)]
pub struct NoiseCacheConfig {
    /// Maximum entries in primary cache
    pub max_entries: usize,
    /// Maximum memory usage in bytes
    pub max_memory: usize,
    /// Time-to-live for entries
    pub ttl: Duration,
    /// Enable spatial indexing
    pub spatial_indexing: bool,
    /// Quick cache size for hot paths
    pub quick_cache_size: usize,
}

impl Default for NoiseCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            max_memory: 64 * 1024 * 1024, // 64MB
            ttl: Duration::from_secs(300),  // 5 minutes
            spatial_indexing: true,
            quick_cache_size: 1000,
        }
    }
}

impl NoiseCache {
    /// Create new noise cache with configuration
    pub fn new(max_entries: usize) -> Self {
        Self::with_config(NoiseCacheConfig {
            max_entries,
            ..Default::default()
        })
    }

    /// Create noise cache with custom configuration
    pub fn with_config(config: NoiseCacheConfig) -> Self {
        // Build moka cache with sophisticated policies
        let primary = CacheBuilder::new(config.max_entries as u64)
            .time_to_live(config.ttl)
            .weigher(|_k: &NoiseCacheKey, v: &CachedNoiseValue| -> u32 {
                (std::mem::size_of_val(v) as u32).max(1)
            })
            .max_capacity(config.max_memory as u64)
            .build();

        let quick = Arc::new(QuickCache::new(config.quick_cache_size));
        let spatial = DashMap::new();
        let stats = Arc::new(std::sync::Mutex::new(CacheStats::default()));

        Self {
            primary,
            quick,
            spatial,
            stats,
            config,
        }
    }

    /// Get noise value from cache
    pub fn get(&self, key: &NoiseCacheKey) -> Option<NoiseResult> {
        // Update stats - will be tracked via hits/misses

        // Try quick cache first for simple lookups
        let fast_hash = key.fast_hash();
        if let Some(value) = self.quick.get(&fast_hash) {
            let mut stats = self.stats.lock().unwrap();
            stats.total_hits += 1;
            return Some(NoiseResult {
                height: value,
                temperature: 0.0, // Quick cache only stores height
                moisture: 0.0,
            });
        }

        // Try primary cache
        if let Some(cached) = self.primary.get(key) {
            // Update access count
            let mut updated = cached.clone();
            updated.access_count += 1;
            self.primary.insert(key.clone(), updated.clone());

            // Update quick cache for future fast access
            self.quick.insert(fast_hash, updated.result.height);

            let mut stats = self.stats.lock().unwrap();
            stats.total_hits += 1;
            stats.memory_usage_bytes = self.calculate_memory_usage() as u64;
            
            return Some(updated.result);
        }

        // Cache miss
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_misses += 1;
        }

        None
    }

    /// Store noise value in cache with spatial indexing
    pub fn put(&self, key: NoiseCacheKey, value: NoiseResult) {
        let cached_value = CachedNoiseValue {
            result: value,
            timestamp_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            access_count: 1,
            coherence_score: self.calculate_coherence_score(&key),
        };

        // Store in primary cache
        self.primary.insert(key.clone(), cached_value);

        // Store in quick cache
        let fast_hash = key.fast_hash();
        self.quick.insert(fast_hash, value.height);

        // Update spatial index
        if self.config.spatial_indexing {
            let spatial_key = (key.x / 10, key.y / 10); // Group nearby coordinates
            self.spatial
                .entry(spatial_key)
                .or_insert_with(Vec::new)
                .push(key);
        }

        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.cache_count = self.primary.entry_count() as usize;
            stats.memory_usage_bytes = self.calculate_memory_usage() as u64;
        }
    }

    /// Invalidate cache entries for a region
    pub fn invalidate_region(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        if !self.config.spatial_indexing {
            return;
        }

        let min_spatial = (min_x as i64 / 10, min_y as i64 / 10);
        let max_spatial = (max_x as i64 / 10, max_y as i64 / 10);

        for x in min_spatial.0..=max_spatial.0 {
            for y in min_spatial.1..=max_spatial.1 {
                if let Some((_, keys)) = self.spatial.remove(&(x, y)) {
                    for key in keys {
                        self.primary.invalidate(&key);
                        let fast_hash = key.fast_hash();
                        self.quick.remove(&fast_hash);
                    }
                }
            }
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let stats = self.stats.lock().unwrap();
        let mut result = stats.clone();
        result.cache_count = self.primary.entry_count() as usize;
        result.memory_usage_bytes = self.calculate_memory_usage() as u64;
        result.update_hit_ratio(); // Use the built-in method
        result
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        self.primary.invalidate_all();
        self.quick.clear();
        self.spatial.clear();
        
        let mut stats = self.stats.lock().unwrap();
        stats.cache_count = 0;
        stats.memory_usage_bytes = 0;
    }

    /// Calculate spatial coherence score for cache priority
    fn calculate_coherence_score(&self, key: &NoiseCacheKey) -> f32 {
        if !self.config.spatial_indexing {
            return 1.0;
        }

        let spatial_key = (key.x / 10, key.y / 10);
        if let Some(neighbors) = self.spatial.get(&spatial_key) {
            // Higher score for areas with more nearby cached values
            (neighbors.len() as f32).ln() + 1.0
        } else {
            1.0
        }
    }

    /// Calculate current memory usage
    fn calculate_memory_usage(&self) -> usize {
        let primary_size = self.primary.entry_count() as usize * 
            (std::mem::size_of::<NoiseCacheKey>() + std::mem::size_of::<CachedNoiseValue>());
        let quick_size = self.config.quick_cache_size * 
            (std::mem::size_of::<u64>() + std::mem::size_of::<f32>());
        let spatial_size = self.spatial.len() * 
            (std::mem::size_of::<(i64, i64)>() + 10 * std::mem::size_of::<NoiseCacheKey>());
        
        primary_size + quick_size + spatial_size
    }

    /// Preload cache with commonly accessed patterns
    pub fn preload_region(&self, center_x: f64, center_y: f64, radius: f64, noise_type: NoiseType, config_hash: u64) {
        let grid_size = 1.0;
        let steps = (radius / grid_size) as i32;
        
        for dx in -steps..=steps {
            for dy in -steps..=steps {
                let x = center_x + dx as f64 * grid_size;
                let y = center_y + dy as f64 * grid_size;
                let distance = ((dx * dx + dy * dy) as f64).sqrt();
                
                if distance <= radius as f64 {
                    let key = NoiseCacheKey::new(x, y, noise_type, config_hash, 0);
                    // This would trigger generation if not cached
                    self.get(&key);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_quantization() {
        let key1 = NoiseCacheKey::new(1.1, 2.1, NoiseType::Simplex, 12345, 0);
        let key2 = NoiseCacheKey::new(1.2, 2.2, NoiseType::Simplex, 12345, 0);
        
        // Should quantize to same values
        assert_eq!(key1.x, key2.x);
        assert_eq!(key1.y, key2.y);
    }

    #[test]
    fn test_cache_put_get() {
        let cache = NoiseCache::new(100);
        let key = NoiseCacheKey::new(0.0, 0.0, NoiseType::Simplex, 12345, 0);
        let value = NoiseResult {
            height: 0.5,
            temperature: 0.3,
            moisture: 0.7,
        };
        
        cache.put(key.clone(), value);
        let retrieved = cache.get(&key);
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().height, 0.5);
    }

    #[test]
    fn test_cache_stats() {
        let cache = NoiseCache::new(100);
        let key = NoiseCacheKey::new(0.0, 0.0, NoiseType::Simplex, 12345, 0);
        
        // Miss
        cache.get(&key);
        
        // Hit
        cache.put(key.clone(), NoiseResult { height: 0.5, temperature: 0.0, moisture: 0.0 });
        cache.get(&key);
        
        let stats = cache.stats();
        assert_eq!(stats.total_hits + stats.total_misses, 2);
        assert_eq!(stats.total_hits, 1);
        assert_eq!(stats.total_misses, 1);
        assert_eq!(stats.hit_ratio, 0.5);
    }
}
