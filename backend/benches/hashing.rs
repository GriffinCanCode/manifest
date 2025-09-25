//! Criterion benchmarks for hashing performance
//!
//! Run with: cargo bench --features=bench

use criterion::{criterion_group, criterion_main};

use manifest::core::benchmarks::*;

criterion_group!(
    hashing_benches,
    bench_component_signature_hashing,
    bench_type_id_hashing, 
    bench_entity_hashing,
    bench_coordinate_hashing,
    bench_hashmap_operations,
    bench_archetype_storage_simulation
);

criterion_main!(hashing_benches);
