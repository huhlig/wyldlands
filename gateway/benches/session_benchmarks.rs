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

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::Arc;
use tokio::runtime::Runtime;
use wyldlands_gateway::pool::ConnectionPool;
use wyldlands_gateway::session::manager::SessionManager;
use wyldlands_gateway::session::{AuthenticatedState, GatewaySession, ProtocolType, SessionState};

/// Create a test database pool for benchmarking
async fn create_bench_pool() -> sqlx::PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/wyldlands_test".to_string());

    sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database")
}

/// Benchmark session creation
fn bench_session_creation(c: &mut Criterion) {
    c.bench_function("session_new", |b| {
        b.iter(|| {
            GatewaySession::new(
                std::hint::black_box(ProtocolType::WebSocket),
                std::hint::black_box("127.0.0.1:8080".to_string()),
            )
        });
    });
}

/// Benchmark session state transitions
fn bench_session_transitions(c: &mut Criterion) {
    let mut session = GatewaySession::new(ProtocolType::Telnet, "127.0.0.1:23".to_string());

    c.bench_function("session_transition", |b| {
        b.iter(|| {
            let _ = session.transition(std::hint::black_box(SessionState::Authenticated(
                AuthenticatedState::Playing,
            )));
            let _ = session.transition(std::hint::black_box(SessionState::Disconnected));
        });
    });
}

/// Benchmark session touch operation
fn bench_session_touch(c: &mut Criterion) {
    let mut session = GatewaySession::new(ProtocolType::WebSocket, "127.0.0.1:8080".to_string());

    c.bench_function("session_touch", |b| {
        b.iter(|| {
            session.touch();
        });
    });
}

/// Benchmark session expiration check
fn bench_session_expiration(c: &mut Criterion) {
    let session = GatewaySession::new(ProtocolType::Telnet, "127.0.0.1:23".to_string());

    c.bench_function("session_is_expired", |b| {
        b.iter(|| session.is_expired(black_box(300)));
    });
}

/// Benchmark SessionManager operations
fn bench_session_manager(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let manager = Arc::new(SessionManager::new(300));

    let mut group = c.benchmark_group("session_manager");

    // Benchmark session creation
    group.bench_function("create_session", |b| {
        b.to_async(&rt).iter(|| async {
            let manager = manager.clone();
            manager
                .create_session(
                    black_box(ProtocolType::WebSocket),
                    black_box("127.0.0.1:8080".to_string()),
                )
                .await
                .expect("Failed to create session")
        });
    });

    // Benchmark session retrieval
    let session_id = rt.block_on(async {
        manager
            .create_session(ProtocolType::Telnet, "127.0.0.1:23".to_string())
            .await
            .expect("Failed to create session")
    });

    group.bench_function("get_session", |b| {
        b.to_async(&rt).iter(|| async {
            let manager = manager.clone();
            manager.get_session(black_box(session_id)).await
        });
    });

    // Benchmark session touch
    group.bench_function("touch_session", |b| {
        b.to_async(&rt).iter(|| async {
            let manager = manager.clone();
            manager
                .touch_session(black_box(session_id))
                .await
                .expect("Failed to touch session")
        });
    });

    group.finish();
}

/// Benchmark ConnectionPool operations
fn bench_connection_pool(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let manager = Arc::new(SessionManager::new(300));
    let pool = Arc::new(ConnectionPool::new(manager.clone()));

    let mut group = c.benchmark_group("connection_pool");

    // Create test sessions
    let session_ids: Vec<_> = rt.block_on(async {
        let mut ids = Vec::new();
        for i in 0..10 {
            let id = manager
                .create_session(ProtocolType::WebSocket, format!("127.0.0.1:808{}", i))
                .await
                .expect("Failed to create session");

            pool.register(id, ProtocolType::WebSocket)
                .await
                .expect("Failed to register connection");

            ids.push(id);
        }
        ids
    });

    // Benchmark connection count
    group.bench_function("connection_count", |b| {
        b.to_async(&rt).iter(|| async {
            let pool = pool.clone();
            pool.connection_count().await
        });
    });

    // Benchmark send to connection
    let test_data = b"Hello, World!".to_vec();
    group.bench_function("send", |b| {
        b.to_async(&rt).iter(|| async {
            let pool = pool.clone();
            let _ = pool
                .send(black_box(session_ids[0]), black_box(test_data.clone()))
                .await;
        });
    });

    // Benchmark broadcast
    group.bench_function("broadcast", |b| {
        b.to_async(&rt).iter(|| async {
            let pool = pool.clone();
            let _ = pool.broadcast(black_box(test_data.clone())).await;
        });
    });

    // Benchmark active sessions
    group.bench_function("active_sessions", |b| {
        b.to_async(&rt).iter(|| async {
            let pool = pool.clone();
            pool.active_sessions().await
        });
    });

    group.finish();
}

/// Benchmark concurrent session operations
fn bench_concurrent_sessions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let manager = Arc::new(SessionManager::new(300));

    let mut group = c.benchmark_group("concurrent_sessions");

    for count in [10, 50, 100, 500].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.to_async(&rt).iter(|| async {
                let manager = manager.clone();
                let mut handles = Vec::new();

                for i in 0..count {
                    let manager = manager.clone();
                    let handle = tokio::spawn(async move {
                        manager
                            .create_session(
                                ProtocolType::WebSocket,
                                format!("127.0.0.1:{}", 8000 + i),
                            )
                            .await
                            .expect("Failed to create session")
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.await.expect("Task failed");
                }
            });
        });
    }

    group.finish();
}

/// Benchmark session cleanup
fn bench_session_cleanup(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let manager = Arc::new(SessionManager::new(300));

    // Create some expired sessions
    rt.block_on(async {
        for i in 0..100 {
            let session_id = manager
                .create_session(ProtocolType::Telnet, format!("127.0.0.1:{}", 2300 + i))
                .await
                .expect("Failed to create session");

            // Make some sessions expired
            if i % 2 == 0 {
                let mut session = manager.get_session(session_id).await.unwrap();
                session.last_activity = chrono::Utc::now() - chrono::Duration::seconds(400);
                manager
                    .update_session(session)
                    .await
                    .expect("Failed to update");
            }
        }
    });

    c.bench_function("cleanup_expired", |b| {
        b.to_async(&rt).iter(|| async {
            let manager = manager.clone();
            manager.cleanup_expired().await.expect("Failed to cleanup")
        });
    });
}

criterion_group!(
    benches,
    bench_session_creation,
    bench_session_transitions,
    bench_session_touch,
    bench_session_expiration,
    bench_session_manager,
    bench_connection_pool,
    bench_concurrent_sessions,
    bench_session_cleanup,
);

criterion_main!(benches);
