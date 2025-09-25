//! Performance benchmarks for core hashing strategies
//!
//! Validates the performance improvements from our optimized hashing
//! compared to standard library hashers in ECS-critical scenarios.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::{HashMap, HashSet};
use std::any::TypeId;
use std::time::Instant;
use bevy_ecs::prelude::Entity;
use glam::IVec2;

use crate::core::hashing::{
    FastHasher, SecureHasher, TypeIdHasher, EntityHasher, CoordinateHasher,
    HashStrategies, collections, FastHashMap, FastHashSet
};

/// Benchmark component signature hashing (most critical for ECS)
fn bench_component_signature_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_signature");
    
    // Create test component type sets of different sizes
    let small_types = vec![
        TypeId::of::<i32>(),
        TypeId::of::<f32>(),
        TypeId::of::<String>(),
    ];
    
    let medium_types = vec![
        TypeId::of::<i32>(), TypeId::of::<f32>(), TypeId::of::<String>(),
        TypeId::of::<Vec<i32>>(), TypeId::of::<HashMap<String, i32>>(),
        TypeId::of::<Option<String>>(), TypeId::of::<Result<i32, String>>(),
        TypeId::of::<Entity>(), TypeId::of::<IVec2>(),
    ];
    
    let large_types = {
        let mut types = medium_types.clone();
        types.extend(vec![
            TypeId::of::<HashSet<String>>(), TypeId::of::<Vec<f64>>(),
            TypeId::of::<&'static str>(), TypeId::of::<Box<dyn std::any::Any>>(),
            TypeId::of::<std::sync::Arc<String>>(), TypeId::of::<std::rc::Rc<i32>>(),
            TypeId::of::<std::cell::RefCell<f32>>(), TypeId::of::<std::sync::Mutex<i64>>(),
            TypeId::of::<std::collections::BTreeMap<String, i32>>(),
        ]);
        types
    };
    
    // Benchmark our optimized signature hashing
    for (name, types) in [
        ("small", &small_types),
        ("medium", &medium_types), 
        ("large", &large_types)
    ] {
        group.bench_with_input(BenchmarkId::new("optimized", name), types, |b, types| {
            b.iter(|| {
                black_box(HashStrategies::hash_type_signature(types))
            })
        });
    }
    
    // Benchmark standard library hashing for comparison
    for (name, types) in [
        ("small", &small_types),
        ("medium", &medium_types), 
        ("large", &large_types)
    ] {
        group.bench_with_input(BenchmarkId::new("standard", name), types, |b, types| {
            b.iter(|| {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                
                let mut hasher = DefaultHasher::new();
                for &type_id in types {
                    type_id.hash(&mut hasher);
                }
                black_box(hasher.finish())
            })
        });
    }
    
    group.finish();
}

/// Benchmark TypeId hashing specifically
fn bench_type_id_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_id_hashing");
    
    let test_types = vec![
        TypeId::of::<i32>(), TypeId::of::<f32>(), TypeId::of::<String>(),
        TypeId::of::<Vec<i32>>(), TypeId::of::<HashMap<String, i32>>(),
        TypeId::of::<Entity>(), TypeId::of::<IVec2>(),
    ];
    
    group.bench_function("optimized", |b| {
        b.iter(|| {
            for &type_id in &test_types {
                black_box(TypeIdHasher::hash(type_id));
            }
        })
    });
    
    group.bench_function("standard", |b| {
        b.iter(|| {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            for &type_id in &test_types {
                let mut hasher = DefaultHasher::new();
                type_id.hash(&mut hasher);
                black_box(hasher.finish());
            }
        })
    });
    
    group.finish();
}

/// Benchmark Entity ID hashing
fn bench_entity_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_hashing");
    
    let test_entities = (0..1000).map(Entity::from_raw).collect::<Vec<_>>();
    
    group.bench_function("optimized", |b| {
        b.iter(|| {
            for &entity in &test_entities {
                black_box(EntityHasher::hash(entity));
            }
        })
    });
    
    group.bench_function("standard", |b| {
        b.iter(|| {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            for &entity in &test_entities {
                let mut hasher = DefaultHasher::new();
                entity.hash(&mut hasher);
                black_box(hasher.finish());
            }
        })
    });
    
    group.finish();
}

/// Benchmark coordinate hashing for spatial queries
fn bench_coordinate_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("coordinate_hashing");
    
    let test_coords = (0..1000)
        .map(|i| IVec2::new(i % 100, i / 100))
        .collect::<Vec<_>>();
    
    group.bench_function("optimized", |b| {
        b.iter(|| {
            for &coord in &test_coords {
                black_box(CoordinateHasher::hash(coord));
            }
        })
    });
    
    group.bench_function("standard", |b| {
        b.iter(|| {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            for &coord in &test_coords {
                let mut hasher = DefaultHasher::new();
                coord.hash(&mut hasher);
                black_box(hasher.finish());
            }
        })
    });
    
    group.finish();
}

/// Benchmark HashMap operations with different hashers
fn bench_hashmap_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashmap_operations");
    
    let test_keys: Vec<TypeId> = vec![
        TypeId::of::<i32>(), TypeId::of::<f32>(), TypeId::of::<String>(),
        TypeId::of::<Vec<i32>>(), TypeId::of::<HashMap<String, i32>>(),
        TypeId::of::<Entity>(), TypeId::of::<IVec2>(),
        TypeId::of::<HashSet<String>>(), TypeId::of::<Option<i32>>(),
        TypeId::of::<Result<String, ()>>(),
    ];
    
    // Benchmark insertions
    group.bench_function("fast_hashmap_insert", |b| {
        b.iter(|| {
            let mut map: FastHashMap<TypeId, usize> = collections::fast_hash_map();
            for (i, &key) in test_keys.iter().enumerate() {
                map.insert(key, i);
            }
            black_box(map);
        })
    });
    
    group.bench_function("std_hashmap_insert", |b| {
        b.iter(|| {
            let mut map: HashMap<TypeId, usize> = HashMap::new();
            for (i, &key) in test_keys.iter().enumerate() {
                map.insert(key, i);
            }
            black_box(map);
        })
    });
    
    // Benchmark lookups
    let fast_map: FastHashMap<TypeId, usize> = {
        let mut map = collections::fast_hash_map();
        for (i, &key) in test_keys.iter().enumerate() {
            map.insert(key, i);
        }
        map
    };
    
    let std_map: HashMap<TypeId, usize> = {
        let mut map = HashMap::new();
        for (i, &key) in test_keys.iter().enumerate() {
            map.insert(key, i);
        }
        map
    };
    
    group.bench_function("fast_hashmap_lookup", |b| {
        b.iter(|| {
            for &key in &test_keys {
                black_box(fast_map.get(&key));
            }
        })
    });
    
    group.bench_function("std_hashmap_lookup", |b| {
        b.iter(|| {
            for &key in &test_keys {
                black_box(std_map.get(&key));
            }
        })
    });
    
    group.finish();
}

/// Benchmark archetype storage simulation
fn bench_archetype_storage_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("archetype_storage");
    
    // Simulate archetype signature hashes
    let signatures: Vec<u64> = (0..1000).map(|i| {
        let types = vec![
            TypeId::of::<i32>(),
            if i % 2 == 0 { TypeId::of::<f32>() } else { TypeId::of::<String>() },
            if i % 3 == 0 { TypeId::of::<Entity>() } else { TypeId::of::<IVec2>() },
        ];
        HashStrategies::hash_type_signature(&types)
    }).collect();
    
    group.bench_function("fast_signature_lookup", |b| {
        b.iter(|| {
            let mut lookup: FastHashMap<u64, usize> = collections::fast_hash_map();
            for (i, &signature) in signatures.iter().enumerate() {
                lookup.insert(signature, i);
            }
            
            // Simulate lookups
            for &signature in &signatures[..100] {
                black_box(lookup.get(&signature));
            }
        })
    });
    
    group.bench_function("std_signature_lookup", |b| {
        b.iter(|| {
            let mut lookup: HashMap<u64, usize> = HashMap::new();
            for (i, &signature) in signatures.iter().enumerate() {
                lookup.insert(signature, i);
            }
            
            // Simulate lookups
            for &signature in &signatures[..100] {
                black_box(lookup.get(&signature));
            }
        })
    });
    
    group.finish();
}

/// Quick runtime performance comparison (not using Criterion for simpler integration)
pub fn quick_performance_test() {
    println!("🚀 Quick Hashing Performance Test");
    println!("==================================");
    
    const ITERATIONS: usize = 100_000;
    
    // Test TypeId hashing
    let test_types = vec![
        TypeId::of::<i32>(), TypeId::of::<f32>(), TypeId::of::<String>(),
        TypeId::of::<Vec<i32>>(), TypeId::of::<Entity>(), TypeId::of::<IVec2>(),
    ];
    
    // Optimized TypeId hashing
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for &type_id in &test_types {
            black_box(TypeIdHasher::hash(type_id));
        }
    }
    let optimized_time = start.elapsed();
    
    // Standard TypeId hashing
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for &type_id in &test_types {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            type_id.hash(&mut hasher);
            black_box(hasher.finish());
        }
    }
    let standard_time = start.elapsed();
    
    let speedup = standard_time.as_nanos() as f64 / optimized_time.as_nanos() as f64;
    
    println!("TypeId Hashing ({} iterations):", ITERATIONS * test_types.len());
    println!("  Optimized: {:?}", optimized_time);
    println!("  Standard:  {:?}", standard_time);
    println!("  Speedup:   {:.2}x faster", speedup);
    println!();
    
    // Test coordinate hashing
    let test_coords: Vec<IVec2> = (0..1000)
        .map(|i| IVec2::new(i % 100, i / 100))
        .collect();
    
    let start = Instant::now();
    for _ in 0..100 {
        for &coord in &test_coords {
            black_box(CoordinateHasher::hash(coord));
        }
    }
    let coord_optimized = start.elapsed();
    
    let start = Instant::now();
    for _ in 0..100 {
        for &coord in &test_coords {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            coord.hash(&mut hasher);
            black_box(hasher.finish());
        }
    }
    let coord_standard = start.elapsed();
    
    let coord_speedup = coord_standard.as_nanos() as f64 / coord_optimized.as_nanos() as f64;
    
    println!("Coordinate Hashing ({} iterations):", 100 * test_coords.len());
    println!("  Optimized: {:?}", coord_optimized);
    println!("  Standard:  {:?}", coord_standard);
    println!("  Speedup:   {:.2}x faster", coord_speedup);
    println!();
    
    println!("✅ Fast hashing is working! Average speedup: {:.2}x", 
             (speedup + coord_speedup) / 2.0);
}

#[cfg(feature = "bench")]
criterion_group!(
    benches,
    bench_component_signature_hashing,
    bench_type_id_hashing,
    bench_entity_hashing,
    bench_coordinate_hashing,
    bench_hashmap_operations,
    bench_archetype_storage_simulation
);

#[cfg(feature = "bench")]
criterion_main!(benches);
