//! Verification system using seahash for fast checksums
//!
//! Provides integrity checking and determinism verification through
//! fast checksum calculation and state validation.

use bevy_ecs::prelude::*;
use parking_lot::RwLock;
use seahash::SeaHasher;
use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    sync::Arc,
    time::Instant,
};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Fast checksum calculation using SeaHash
#[derive(Debug, Clone)]
pub struct StateChecksum {
    /// Primary checksum of game state
    pub state_hash: u64,
    /// Entity count checksum
    pub entity_hash: u64,
    /// Resource checksum
    pub resource_hash: u64,
    /// Component checksum
    pub component_hash: u64,
    /// Combined checksum for quick comparison
    pub combined_hash: u64,
    /// Tick when checksum was calculated
    pub tick: u64,
}

impl StateChecksum {
    /// Create new empty checksum
    pub fn new(tick: u64) -> Self {
        Self {
            state_hash: 0,
            entity_hash: 0,
            resource_hash: 0,
            component_hash: 0,
            combined_hash: 0,
            tick,
        }
    }

    /// Calculate combined hash from individual hashes
    pub fn calculate_combined(&mut self) {
        let mut hasher = SeaHasher::new();
        self.state_hash.hash(&mut hasher);
        self.entity_hash.hash(&mut hasher);
        self.resource_hash.hash(&mut hasher);
        self.component_hash.hash(&mut hasher);
        self.combined_hash = hasher.finish();
    }

    /// Compare with another checksum
    pub fn matches(&self, other: &StateChecksum) -> bool {
        self.combined_hash == other.combined_hash
    }

    /// Get detailed comparison with another checksum
    pub fn detailed_compare(&self, other: &StateChecksum) -> ChecksumComparison {
        ChecksumComparison {
            state_matches: self.state_hash == other.state_hash,
            entity_matches: self.entity_hash == other.entity_hash,
            resource_matches: self.resource_hash == other.resource_hash,
            component_matches: self.component_hash == other.component_hash,
            combined_matches: self.combined_hash == other.combined_hash,
        }
    }
}

/// Detailed comparison between two checksums
#[derive(Debug, Clone)]
pub struct ChecksumComparison {
    pub state_matches: bool,
    pub entity_matches: bool,
    pub resource_matches: bool,
    pub component_matches: bool,
    pub combined_matches: bool,
}

impl ChecksumComparison {
    /// Check if all components match
    pub fn all_match(&self) -> bool {
        self.state_matches && self.entity_matches && self.resource_matches && self.component_matches
    }

    /// Get list of mismatched components
    pub fn mismatches(&self) -> Vec<String> {
        let mut mismatches = Vec::new();
        if !self.state_matches { mismatches.push("state".to_string()); }
        if !self.entity_matches { mismatches.push("entities".to_string()); }
        if !self.resource_matches { mismatches.push("resources".to_string()); }
        if !self.component_matches { mismatches.push("components".to_string()); }
        mismatches
    }
}

/// Verification system for state integrity checking
#[derive(Debug)]
pub struct VerificationSystem {
    /// Historical checksums by tick
    checksums: Arc<RwLock<BTreeMap<u64, StateChecksum>>>,
    /// Expected checksums for determinism verification
    expected_checksums: Arc<RwLock<BTreeMap<u64, StateChecksum>>>,
    /// Initial seed for reproducibility
    initial_seed: u64,
    /// Verification statistics
    stats: Arc<RwLock<VerificationStats>>,
    /// Enable verification (can be disabled for performance)
    enabled: bool,
}

impl VerificationSystem {
    /// Create new verification system
    pub fn new(seed: u64) -> Self {
        Self {
            checksums: Arc::new(RwLock::new(BTreeMap::new())),
            expected_checksums: Arc::new(RwLock::new(BTreeMap::new())),
            initial_seed: seed,
            stats: Arc::new(RwLock::new(VerificationStats::default())),
            enabled: true,
        }
    }

    /// Calculate checksum for world state at given tick
    pub fn calculate_checksum(&self, world: &World, tick: u64) -> u64 {
        if !self.enabled {
            return 0;
        }

        let start = Instant::now();
        let mut checksum = StateChecksum::new(tick);

        // Hash entities - simplified implementation
        checksum.entity_hash = self.hash_entities(world);
        
        // Hash resources
        checksum.resource_hash = self.hash_resources(world);
        
        // Hash components - would need proper ECS integration
        checksum.component_hash = self.hash_components(world);
        
        // Calculate state hash from seed and tick
        let mut state_hasher = SeaHasher::new();
        self.initial_seed.hash(&mut state_hasher);
        tick.hash(&mut state_hasher);
        checksum.state_hash = state_hasher.finish();
        
        // Calculate combined hash
        checksum.calculate_combined();

        // Store checksum
        self.checksums.write().insert(tick, checksum.clone());

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.checksums_calculated += 1;
            stats.total_calculation_time += start.elapsed();
        }

        debug!("Calculated checksum {} for tick {}", checksum.combined_hash, tick);
        checksum.combined_hash
    }

    /// Verify checksum matches expected value
    pub fn verify_checksum(&self, tick: u64, expected: u64) -> Result<(), VerificationError> {
        if !self.enabled {
            return Ok(());
        }

        let checksums = self.checksums.read();
        if let Some(checksum) = checksums.get(&tick) {
            if checksum.combined_hash != expected {
                let mut stats = self.stats.write();
                stats.verification_failures += 1;
                
                return Err(VerificationError::ChecksumMismatch {
                    tick,
                    expected,
                    actual: checksum.combined_hash,
                });
            }
            
            let mut stats = self.stats.write();
            stats.verifications_passed += 1;
            
            debug!("Checksum verification passed for tick {}", tick);
            Ok(())
        } else {
            Err(VerificationError::ChecksumNotFound(tick))
        }
    }

    /// Set expected checksum for future verification
    pub fn set_expected_checksum(&self, tick: u64, checksum: StateChecksum) {
        self.expected_checksums.write().insert(tick, checksum);
    }

    /// Verify against expected checksum
    pub fn verify_against_expected(&self, tick: u64) -> Result<ChecksumComparison, VerificationError> {
        let checksums = self.checksums.read();
        let expected_checksums = self.expected_checksums.read();

        let actual = checksums.get(&tick)
            .ok_or(VerificationError::ChecksumNotFound(tick))?;
        
        let expected = expected_checksums.get(&tick)
            .ok_or(VerificationError::ExpectedChecksumNotFound(tick))?;

        let comparison = actual.detailed_compare(expected);
        
        if !comparison.all_match() {
            let mut stats = self.stats.write();
            stats.verification_failures += 1;
            
            warn!("Determinism verification failed for tick {}: mismatches in {:?}", 
                  tick, comparison.mismatches());
        } else {
            let mut stats = self.stats.write();
            stats.verifications_passed += 1;
        }

        Ok(comparison)
    }

    /// Get checksum for specific tick
    pub fn get_checksum(&self, tick: u64) -> Option<StateChecksum> {
        self.checksums.read().get(&tick).cloned()
    }

    /// Get verification statistics
    pub fn stats(&self) -> VerificationStats {
        self.stats.read().clone()
    }

    /// Get number of verified checksums
    pub fn verified_count(&self) -> u64 {
        self.stats.read().checksums_calculated
    }

    /// Enable or disable verification
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        info!("Verification system {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Clear stored checksums
    pub fn clear(&self) {
        self.checksums.write().clear();
        self.expected_checksums.write().clear();
        *self.stats.write() = VerificationStats::default();
        info!("Cleared verification history");
    }

    /// Export checksums for determinism testing
    pub fn export_checksums(&self) -> Vec<(u64, u64)> {
        self.checksums
            .read()
            .iter()
            .map(|(&tick, checksum)| (tick, checksum.combined_hash))
            .collect()
    }

    /// Import checksums for comparison
    pub fn import_expected_checksums(&self, checksums: Vec<(u64, u64)>) {
        let mut expected = self.expected_checksums.write();
        for (tick, hash) in checksums {
            let mut checksum = StateChecksum::new(tick);
            checksum.combined_hash = hash;
            expected.insert(tick, checksum);
        }
        info!("Imported {} expected checksums", expected.len());
    }

    /// Fast hash of entities - simplified implementation
    fn hash_entities(&self, _world: &World) -> u64 {
        let mut hasher = SeaHasher::new();
        
        // This would need proper ECS integration to hash all entities
        // For now, return a placeholder based on world state
        0u64.hash(&mut hasher); // Placeholder
        
        hasher.finish()
    }

    /// Fast hash of resources
    fn hash_resources(&self, _world: &World) -> u64 {
        let mut hasher = SeaHasher::new();
        
        // This would need proper resource iteration and hashing
        // For now, return a placeholder
        0u64.hash(&mut hasher); // Placeholder
        
        hasher.finish()
    }

    /// Fast hash of components
    fn hash_components(&self, _world: &World) -> u64 {
        let mut hasher = SeaHasher::new();
        
        // This would need proper component iteration and hashing
        // For now, return a placeholder
        0u64.hash(&mut hasher); // Placeholder
        
        hasher.finish()
    }
}

/// Verification statistics
#[derive(Debug, Default, Clone)]
pub struct VerificationStats {
    pub checksums_calculated: u64,
    pub verifications_passed: u64,
    pub verification_failures: u64,
    pub total_calculation_time: std::time::Duration,
}

impl VerificationStats {
    /// Get average calculation time per checksum
    pub fn average_calculation_time(&self) -> std::time::Duration {
        if self.checksums_calculated > 0 {
            self.total_calculation_time / self.checksums_calculated as u32
        } else {
            std::time::Duration::ZERO
        }
    }

    /// Get verification success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.verifications_passed + self.verification_failures;
        if total > 0 {
            self.verifications_passed as f64 / total as f64
        } else {
            0.0
        }
    }
}

/// Verification errors
#[derive(Error, Debug)]
pub enum VerificationError {
    #[error("Checksum not found for tick {0}")]
    ChecksumNotFound(u64),
    #[error("Expected checksum not found for tick {0}")]
    ExpectedChecksumNotFound(u64),
    #[error("Checksum mismatch at tick {tick}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        tick: u64,
        expected: u64,
        actual: u64,
    },
    #[error("State extraction failed: {0}")]
    StateExtractionFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_system_creation() {
        let system = VerificationSystem::new(42);
        assert_eq!(system.initial_seed, 42);
        assert!(system.enabled);
    }

    #[test]
    fn test_state_checksum() {
        let mut checksum = StateChecksum::new(100);
        checksum.state_hash = 123;
        checksum.entity_hash = 456;
        checksum.resource_hash = 789;
        checksum.component_hash = 101112;
        
        checksum.calculate_combined();
        assert_ne!(checksum.combined_hash, 0);
    }

    #[test]
    fn test_checksum_comparison() {
        let checksum1 = StateChecksum {
            state_hash: 123,
            entity_hash: 456,
            resource_hash: 789,
            component_hash: 101112,
            combined_hash: 999,
            tick: 100,
        };

        let checksum2 = StateChecksum {
            state_hash: 123,
            entity_hash: 999, // Different
            resource_hash: 789,
            component_hash: 101112,
            combined_hash: 888,
            tick: 100,
        };

        let comparison = checksum1.detailed_compare(&checksum2);
        assert!(comparison.state_matches);
        assert!(!comparison.entity_matches);
        assert!(comparison.resource_matches);
        assert!(comparison.component_matches);
        assert!(!comparison.all_match());

        let mismatches = comparison.mismatches();
        assert!(mismatches.contains(&"entities".to_string()));
    }

    #[test]
    fn test_verification_stats() {
        let mut stats = VerificationStats::default();
        stats.checksums_calculated = 10;
        stats.verifications_passed = 8;
        stats.verification_failures = 2;
        stats.total_calculation_time = std::time::Duration::from_millis(100);

        assert_eq!(stats.success_rate(), 0.8);
        assert_eq!(stats.average_calculation_time(), std::time::Duration::from_millis(10));
    }
}
