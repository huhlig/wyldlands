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

//! Wyldlands Gateway Library
//!
//! This library provides the core functionality for the Wyldlands gateway server,
//! including session management, connection pooling, and sidechannel adapters.

//pub mod auth;
pub mod properties;
pub mod config;
pub mod context;
pub mod grpc;
pub mod pool;
pub mod sidechannel;
pub mod reconnection;
pub mod server;
pub mod session;

// Re-export commonly used types
pub use context::ServerContext;
pub use grpc::{GatewayRpcServer, RpcClientManager};
pub use pool::ConnectionPool;
pub use sidechannel::msdp;
pub use reconnection::{ReconnectionManager, ReconnectionToken};
pub use session::{GatewaySession, ProtocolType, SessionState};


