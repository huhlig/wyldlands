//
// Copyright 2025-2026 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//! Performance benchmarks for the memory system
//!
//! Run with: cargo bench --bench memory_benchmarks

use chrono::Utc;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use sqlx::PgPool;
use std::time::Duration;
use wyldlands_server::ecs::components::EntityId;
use wyldlands_server::ecs::memory::{MemoryBatchItem, MemoryKind, MemoryResource, MemoryTagMode};

/// Setup test database connection
async fn setup_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/wyldlands_test".to_string());

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Benchmark memory retention (single)
fn bench_retain_single(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(setup_db());
    let memory = MemoryResource::new(pool);
    let entity_id = EntityId::from_uuid(uuid::Uuid::new_v4());

    c.bench_function("retain_single", |b| {
        b.to_async(&rt).iter_batched(
            || (memory.clone(), entity_id),
            |(mut memory, entity_id)| async move {
                memory
                    .retain(
                        std::hint::black_box(entity_id),
                        std::hint::black_box(MemoryKind::Experience),
                        std::hint::black_box("Test memory content for benchmarking"),
                        std::hint::black_box(Utc::now()),
                        Some("benchmark context"),
                        [("key", "value")],
                        [],
                        ["benchmark"],
                    )
                    .await
                    .unwrap()
            },
            BatchSize::SmallInput,
        );
    });
}

/// Benchmark batch memory retention
fn bench_retain_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(setup_db());
    let memory = MemoryResource::new(pool);
    let entity_id = EntityId::from_uuid(uuid::Uuid::new_v4());

    let mut group = c.benchmark_group("retain_batch");

    for size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter_batched(
                || (memory.clone(), entity_id, size),
                |(mut memory, entity_id, size)| async move {
                    let items: Vec<MemoryBatchItem> = (0..size)
                        .map(|i| {
                            MemoryBatchItem::new(
                                entity_id,
                                MemoryKind::Experience,
                                format!("Batch memory content {}", i),
                            )
                            .with_tag("benchmark".to_string())
                        })
                        .collect();

                    memory
                        .retain_batch(std::hint::black_box(items))
                        .await
                        .unwrap()
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark memory recall without cache
fn bench_recall_cold(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(setup_db());
    let mut memory = MemoryResource::new(pool);
    let entity_id = EntityId::from_uuid(uuid::Uuid::new_v4());

    // Setup: Create some memories
    rt.block_on(async {
        for i in 0..100 {
            memory
                .retain(
                    entity_id,
                    MemoryKind::Experience,
                    &format!("Memory content for recall benchmark {}", i),
                    Utc::now(),
                    None,
                    [],
                    [],
                    ["recall_bench"],
                )
                .await
                .unwrap();
        }
    });

    c.bench_function("recall_cold", |b| {
        b.to_async(&rt).iter(|| async {
            // Clear cache before each iteration
            memory.clear_all_caches().await;

            memory
                .recall(
                    std::hint::black_box(entity_id),
                    std::hint::black_box("recall benchmark query"),
                    std::hint::black_box([MemoryKind::Experience]),
                    std::hint::black_box(["recall_bench"]),
                    std::hint::black_box(MemoryTagMode::Any),
                )
                .await
                .unwrap()
        });
    });
}

/// Benchmark memory recall with cache (warm)
fn bench_recall_warm(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(setup_db());
    let mut memory = MemoryResource::new(pool);
    let entity_id = EntityId::from_uuid(uuid::Uuid::new_v4());

    // Setup: Create some memories
    rt.block_on(async {
        for i in 0..100 {
            memory
                .retain(
                    entity_id,
                    MemoryKind::Experience,
                    &format!("Memory content for recall benchmark {}", i),
                    Utc::now(),
                    None,
                    [],
                    [],
                    ["recall_bench"],
                )
                .await
                .unwrap();
        }

        // Warm up cache
        memory
            .recall(
                entity_id,
                "recall benchmark query",
                [MemoryKind::Experience],
                ["recall_bench"],
                MemoryTagMode::Any,
            )
            .await
            .unwrap();
    });

    c.bench_function("recall_warm", |b| {
        b.to_async(&rt).iter(|| async {
            memory
                .recall(
                    std::hint::black_box(entity_id),
                    std::hint::black_box("recall benchmark query"),
                    std::hint::black_box([MemoryKind::Experience]),
                    std::hint::black_box(["recall_bench"]),
                    std::hint::black_box(MemoryTagMode::Any),
                )
                .await
                .unwrap()
        });
    });
}

/// Benchmark embedding generation (single)
fn bench_embedding_single(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(setup_db());
    let memory = MemoryResource::new(pool);

    c.bench_function("embedding_single", |b| {
        b.to_async(&rt).iter(|| async {
            memory
                .generate_embeddings_batch(std::hint::black_box(&[String::from(
                    "Test text for embedding generation",
                )]))
                .await
                .unwrap()
        });
    });
}

/// Benchmark batch embedding generation
fn bench_embedding_batch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(setup_db());
    let memory = MemoryResource::new(pool);

    let mut group = c.benchmark_group("embedding_batch");

    for size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let texts: Vec<String> = (0..size)
                    .map(|i| format!("Test text for batch embedding {}", i))
                    .collect();

                memory
                    .generate_embeddings_batch(std::hint::black_box(&texts))
                    .await
                    .unwrap()
            });
        });
    }

    group.finish();
}

/// Benchmark cache operations
fn bench_cache_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = rt.block_on(setup_db());
    let memory = MemoryResource::new(pool);
    let entity_id = EntityId::from_uuid(uuid::Uuid::new_v4());

    // Setup: Create a memory
    let memory_id = rt.block_on(async {
        let mut mem = memory.clone();
        mem.retain(
            entity_id,
            MemoryKind::Experience,
            "Cache benchmark memory",
            Utc::now(),
            None,
            [],
            [],
            ["cache_bench"],
        )
        .await
        .unwrap()
    });

    let mut group = c.benchmark_group("cache");

    // Benchmark cache hit
    group.bench_function("get_memory_cached", |b| {
        b.to_async(&rt).iter(|| async {
            memory
                .get_memory(std::hint::black_box(memory_id.clone()))
                .await
                .unwrap()
        });
    });

    // Benchmark cache miss
    group.bench_function("get_memory_uncached", |b| {
        b.to_async(&rt).iter(|| async {
            memory.clear_all_caches().await;
            memory
                .get_memory(std::hint::black_box(memory_id.clone()))
                .await
                .unwrap()
        });
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(50);
    targets =
        bench_retain_single,
        bench_retain_batch,
        bench_recall_cold,
        bench_recall_warm,
        bench_embedding_single,
        bench_embedding_batch,
        bench_cache_operations
}

criterion_main!(benches);


