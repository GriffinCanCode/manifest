//! High-performance hashing strategies using Blake3 and XXHash
//!
//! Provides specialized hashers optimized for different use cases:
//! - XXHash for ultra-fast non-cryptographic hashing (ECS, spatial queries)
//! - Blake3 for secure hashing when cryptographic properties are needed
//! - Specialized hashers for common types (TypeId, Entity, coordinates)

use blake3;
use std::hash::{BuildHasher, Hash, Hasher};
use std::any::TypeId;
use xxhash_rust::xxh3::{Xxh3, xxh3_64, xxh3_64_with_seed};
use bevy_ecs::prelude::Entity;
use glam::IVec2;

/// Fast non-cryptographic hasher using XXHash3
#[derive(Clone, Default)]
pub struct FastHasher {
    hasher: Xxh3,
}

impl FastHasher {
    /// Create new fast hasher
    #[inline]
    pub fn new() -> Self {
        Self {
            hasher: Xxh3::new(),
        }
    }

    /// Create fast hasher with seed for deterministic hashing
    #[inline]
    pub fn with_seed(seed: u64) -> Self {
        Self {
            hasher: Xxh3::with_seed(seed),
        }
    }

    /// Hash a single value quickly
    #[inline]
    pub fn hash_one<T: Hash>(value: &T) -> u64 {
        let mut hasher = Self::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash a single value with seed
    #[inline]
    pub fn hash_one_with_seed<T: Hash>(value: &T, seed: u64) -> u64 {
        let mut hasher = Self::with_seed(seed);
        value.hash(&mut hasher);
        hasher.finish()
    }
}

impl Hasher for FastHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hasher.digest()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.hasher.update(bytes);
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.hasher.update(&[i]);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.hasher.update(&i.to_ne_bytes());
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.hasher.update(&i.to_ne_bytes());
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.hasher.update(&i.to_ne_bytes());
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.hasher.update(&i.to_ne_bytes());
    }

    #[inline]
    fn write_i8(&mut self, i: i8) {
        self.hasher.update(&i.to_ne_bytes());
    }

    #[inline]
    fn write_i16(&mut self, i: i16) {
        self.hasher.update(&i.to_ne_bytes());
    }

    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.hasher.update(&i.to_ne_bytes());
    }

    #[inline]
    fn write_i64(&mut self, i: i64) {
        self.hasher.update(&i.to_ne_bytes());
    }

    #[inline]
    fn write_isize(&mut self, i: isize) {
        self.hasher.update(&i.to_ne_bytes());
    }
}

/// Build hasher for XXHash3
#[derive(Clone, Debug, Default)]
pub struct FastHasherBuilder {
    seed: Option<u64>,
}

impl FastHasherBuilder {
    /// Create new builder
    #[inline]
    pub fn new() -> Self {
        Self { seed: None }
    }

    /// Create builder with seed
    #[inline]
    pub fn with_seed(seed: u64) -> Self {
        Self { seed: Some(seed) }
    }
}

impl BuildHasher for FastHasherBuilder {
    type Hasher = FastHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        match self.seed {
            Some(seed) => FastHasher::with_seed(seed),
            None => FastHasher::new(),
        }
    }
}

/// Secure cryptographic hasher using Blake3
#[derive(Clone, Debug, Default)]
pub struct SecureHasher {
    hasher: blake3::Hasher,
}

impl SecureHasher {
    /// Create new secure hasher
    #[inline]
    pub fn new() -> Self {
        Self {
            hasher: blake3::Hasher::new(),
        }
    }

    /// Create secure hasher with key
    #[inline]
    pub fn with_key(key: &[u8; 32]) -> Self {
        Self {
            hasher: blake3::Hasher::new_keyed(key),
        }
    }

    /// Hash a single value securely
    #[inline]
    pub fn hash_one<T: Hash>(value: &T) -> [u8; 32] {
        let mut hasher = Self::new();
        value.hash(&mut hasher);
        hasher.finalize_fixed()
    }

    /// Get 64-bit hash output (truncated)
    #[inline]
    pub fn finalize_u64(&self) -> u64 {
        let hash = self.hasher.finalize();
        u64::from_ne_bytes(hash.as_bytes()[0..8].try_into().unwrap())
    }

    /// Get full hash output
    #[inline]
    pub fn finalize_fixed(&self) -> [u8; 32] {
        self.hasher.finalize().into()
    }
}

impl Hasher for SecureHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.finalize_u64()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.hasher.update(bytes);
    }
}

/// Build hasher for Blake3
#[derive(Clone, Debug, Default)]
pub struct SecureHasherBuilder {
    key: Option<[u8; 32]>,
}

impl SecureHasherBuilder {
    /// Create new builder
    #[inline]
    pub fn new() -> Self {
        Self { key: None }
    }

    /// Create builder with key
    #[inline]
    pub fn with_key(key: [u8; 32]) -> Self {
        Self { key: Some(key) }
    }
}

impl BuildHasher for SecureHasherBuilder {
    type Hasher = SecureHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        match &self.key {
            Some(key) => SecureHasher::with_key(key),
            None => SecureHasher::new(),
        }
    }
}

/// Specialized hasher for TypeId (most critical for ECS performance)
pub struct TypeIdHasher;

impl TypeIdHasher {
    /// Hash TypeId using XXHash3 optimized for TypeId representation
    #[inline]
    pub fn hash(type_id: TypeId) -> u64 {
        // Use the fastest hashing approach for TypeId
        use std::hash::{Hash, Hasher};
        let mut hasher = Xxh3::new();
        type_id.hash(&mut hasher);
        hasher.digest()
    }

    /// Hash TypeId with seed
    #[inline]
    pub fn hash_with_seed(type_id: TypeId, seed: u64) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = Xxh3::with_seed(seed);
        type_id.hash(&mut hasher);
        hasher.digest()
    }
}

/// Specialized hasher for Entity IDs
pub struct EntityHasher;

impl EntityHasher {
    /// Hash Entity using XXHash3
    #[inline]
    pub fn hash(entity: Entity) -> u64 {
        // Entity is essentially a u32 index + u32 generation
        let (index, generation) = (entity.index(), entity.generation());
        let combined = ((generation as u64) << 32) | (index as u64);
        xxh3_64(&combined.to_ne_bytes())
    }

    /// Hash Entity with seed
    #[inline]
    pub fn hash_with_seed(entity: Entity, seed: u64) -> u64 {
        let (index, generation) = (entity.index(), entity.generation());
        let combined = ((generation as u64) << 32) | (index as u64);
        xxh3_64_with_seed(&combined.to_ne_bytes(), seed)
    }
}

/// Specialized hasher for spatial coordinates (IVec2)
pub struct CoordinateHasher;

impl CoordinateHasher {
    /// Hash coordinates using XXHash3
    #[inline]
    pub fn hash(pos: IVec2) -> u64 {
        // Combine x and y into a single u64 for efficient hashing
        let combined = ((pos.y as u64) << 32) | (pos.x as u64);
        xxh3_64(&combined.to_ne_bytes())
    }

    /// Hash coordinates with seed
    #[inline]
    pub fn hash_with_seed(pos: IVec2, seed: u64) -> u64 {
        let combined = ((pos.y as u64) << 32) | (pos.x as u64);
        xxh3_64_with_seed(&combined.to_ne_bytes(), seed)
    }

    /// Hash hex coordinates efficiently for hex-grid games
    #[inline]
    pub fn hash_hex(q: i32, r: i32) -> u64 {
        let combined = ((r as u64) << 32) | (q as u64);
        xxh3_64(&combined.to_ne_bytes())
    }

    /// Hash hex coordinates with seed
    #[inline]
    pub fn hash_hex_with_seed(q: i32, r: i32, seed: u64) -> u64 {
        let combined = ((r as u64) << 32) | (q as u64);
        xxh3_64_with_seed(&combined.to_ne_bytes(), seed)
    }
}

/// Collection of optimized hash functions for common use cases
pub struct HashStrategies;

impl HashStrategies {
    /// Hash a slice of TypeIds efficiently (for component signatures)
    #[inline]
    pub fn hash_type_signature(types: &[TypeId]) -> u64 {
        if types.is_empty() {
            return 0;
        }

        let mut hasher = Xxh3::new();
        for &type_id in types {
            // Hash TypeId efficiently using its Hash implementation
            use std::hash::{Hash, Hasher as _};
            
            // Create a temporary hasher to get the hash value, then feed it to our main hasher
            let mut temp_hasher = Xxh3::new();
            type_id.hash(&mut temp_hasher);
            let type_hash = temp_hasher.digest();
            hasher.update(&type_hash.to_ne_bytes());
        }
        hasher.digest()
    }

    /// Hash string slice efficiently
    #[inline]
    pub fn hash_string(s: &str) -> u64 {
        xxh3_64(s.as_bytes())
    }

    /// Hash bytes efficiently
    #[inline]
    pub fn hash_bytes(bytes: &[u8]) -> u64 {
        xxh3_64(bytes)
    }

    /// Combine multiple hashes efficiently
    #[inline]
    pub fn combine_hashes(hashes: &[u64]) -> u64 {
        if hashes.is_empty() {
            return 0;
        }

        let mut hasher = Xxh3::new();
        for &hash in hashes {
            hasher.update(&hash.to_ne_bytes());
        }
        hasher.digest()
    }
}

/// Type aliases for common hash map types using fast hashers
pub type FastHashMap<K, V> = std::collections::HashMap<K, V, FastHasherBuilder>;
pub type SecureHashMap<K, V> = std::collections::HashMap<K, V, SecureHasherBuilder>;
pub type FastHashSet<T> = std::collections::HashSet<T, FastHasherBuilder>;
pub type SecureHashSet<T> = std::collections::HashSet<T, SecureHasherBuilder>;

/// Convenience functions for creating optimized collections
pub mod collections {
    use super::*;

    /// Create a new FastHashMap
    #[inline]
    pub fn fast_hash_map<K, V>() -> FastHashMap<K, V> {
        FastHashMap::with_hasher(FastHasherBuilder::new())
    }

    /// Create a new FastHashMap with capacity
    #[inline]
    pub fn fast_hash_map_with_capacity<K, V>(capacity: usize) -> FastHashMap<K, V> {
        FastHashMap::with_capacity_and_hasher(capacity, FastHasherBuilder::new())
    }

    /// Create a new FastHashSet
    #[inline]
    pub fn fast_hash_set<T>() -> FastHashSet<T> {
        FastHashSet::with_hasher(FastHasherBuilder::new())
    }

    /// Create a new FastHashSet with capacity
    #[inline]
    pub fn fast_hash_set_with_capacity<T>(capacity: usize) -> FastHashSet<T> {
        FastHashSet::with_capacity_and_hasher(capacity, FastHasherBuilder::new())
    }

    /// Create a new SecureHashMap
    #[inline]
    pub fn secure_hash_map<K, V>() -> SecureHashMap<K, V> {
        SecureHashMap::with_hasher(SecureHasherBuilder::new())
    }

    /// Create a new SecureHashSet
    #[inline]
    pub fn secure_hash_set<T>() -> SecureHashSet<T> {
        SecureHashSet::with_hasher(SecureHasherBuilder::new())
    }
}

/// Specialized HashMap builders that choose the optimal hasher for the key type
pub mod specialized_maps {
    use super::*;
    
    /// Create a HashMap optimized specifically for TypeId keys
    pub fn type_id_map<V>() -> FastHashMap<TypeId, V> {
        collections::fast_hash_map()
    }
    
    /// Create a HashMap optimized specifically for Entity keys
    pub fn entity_map<V>() -> FastHashMap<Entity, V> {
        collections::fast_hash_map()
    }
    
    /// Create a HashMap optimized specifically for coordinate keys
    pub fn coordinate_map<V>() -> FastHashMap<IVec2, V> {
        collections::fast_hash_map()
    }
    
    /// Create a HashMap optimized for string keys
    pub fn string_map<V>() -> FastHashMap<String, V> {
        collections::fast_hash_map()
    }
    
    /// Create a HashMap with capacity optimized for TypeId keys
    pub fn type_id_map_with_capacity<V>(capacity: usize) -> FastHashMap<TypeId, V> {
        collections::fast_hash_map_with_capacity(capacity)
    }
    
    /// Create a HashMap with capacity optimized for Entity keys
    pub fn entity_map_with_capacity<V>(capacity: usize) -> FastHashMap<Entity, V> {
        collections::fast_hash_map_with_capacity(capacity)
    }
    
    /// Create a HashMap with capacity optimized for coordinate keys
    pub fn coordinate_map_with_capacity<V>(capacity: usize) -> FastHashMap<IVec2, V> {
        collections::fast_hash_map_with_capacity(capacity)
    }
}

/// Specialized HashSet builders that choose the optimal hasher for the element type
pub mod specialized_sets {
    use super::*;
    
    /// Create a HashSet optimized specifically for TypeId elements
    pub fn type_id_set() -> FastHashSet<TypeId> {
        collections::fast_hash_set()
    }
    
    /// Create a HashSet optimized specifically for Entity elements
    pub fn entity_set() -> FastHashSet<Entity> {
        collections::fast_hash_set()
    }
    
    /// Create a HashSet optimized specifically for coordinate elements
    pub fn coordinate_set() -> FastHashSet<IVec2> {
        collections::fast_hash_set()
    }
    
    /// Create a HashSet optimized for string elements
    pub fn string_set() -> FastHashSet<String> {
        collections::fast_hash_set()
    }
    
    /// Create a HashSet with capacity optimized for TypeId elements
    pub fn type_id_set_with_capacity(capacity: usize) -> FastHashSet<TypeId> {
        collections::fast_hash_set_with_capacity(capacity)
    }
    
    /// Create a HashSet with capacity optimized for Entity elements
    pub fn entity_set_with_capacity(capacity: usize) -> FastHashSet<Entity> {
        collections::fast_hash_set_with_capacity(capacity)
    }
    
    /// Create a HashSet with capacity optimized for coordinate elements
    pub fn coordinate_set_with_capacity(capacity: usize) -> FastHashSet<IVec2> {
        collections::fast_hash_set_with_capacity(capacity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    use std::time::Instant;

    #[test]
    fn fast_hasher_basic() {
        let mut hasher = FastHasher::new();
        hasher.write_u64(12345);
        let hash = hasher.finish();
        assert_ne!(hash, 0);
    }

    #[test]
    fn secure_hasher_basic() {
        let mut hasher = SecureHasher::new();
        hasher.write_u64(12345);
        let hash = hasher.finish();
        assert_ne!(hash, 0);
    }

    #[test]
    fn type_id_hasher_consistency() {
        let type_id = TypeId::of::<String>();
        let hash1 = TypeIdHasher::hash(type_id);
        let hash2 = TypeIdHasher::hash(type_id);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn entity_hasher_consistency() {
        let entity = Entity::from_raw(12345);
        let hash1 = EntityHasher::hash(entity);
        let hash2 = EntityHasher::hash(entity);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn coordinate_hasher_consistency() {
        let pos = IVec2::new(100, 200);
        let hash1 = CoordinateHasher::hash(pos);
        let hash2 = CoordinateHasher::hash(pos);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn hash_strategies_signature() {
        let types = vec![
            TypeId::of::<String>(),
            TypeId::of::<i32>(),
            TypeId::of::<f64>(),
        ];
        let hash = HashStrategies::hash_type_signature(&types);
        assert_ne!(hash, 0);
    }

    #[test]
    fn fast_collections_creation() {
        let _map: FastHashMap<String, i32> = collections::fast_hash_map();
        let _set: FastHashSet<String> = collections::fast_hash_set();
        
        let _cap_map: FastHashMap<String, i32> = collections::fast_hash_map_with_capacity(100);
        let _cap_set: FastHashSet<String> = collections::fast_hash_set_with_capacity(100);
    }

    #[cfg(feature = "bench")]
    #[test]
    fn benchmark_hashers() {
        const ITERATIONS: usize = 1_000_000;
        let test_data: Vec<u64> = (0..ITERATIONS).map(|i| i as u64).collect();

        // Benchmark FastHasher (XXHash3)
        let start = Instant::now();
        for &value in &test_data {
            let _hash = FastHasher::hash_one(&value);
        }
        let xxhash_time = start.elapsed();

        // Benchmark DefaultHasher
        let start = Instant::now();
        for &value in &test_data {
            use std::collections::hash_map::DefaultHasher;
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            let _hash = hasher.finish();
        }
        let default_time = start.elapsed();

        println!("XXHash3 time: {:?}", xxhash_time);
        println!("DefaultHasher time: {:?}", default_time);
        println!("XXHash3 speedup: {:.2}x", default_time.as_nanos() as f64 / xxhash_time.as_nanos() as f64);
    }
}
