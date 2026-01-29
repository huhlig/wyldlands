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

//! Wyldlands Gateway - Connection and session management for the MUD server

pub mod admin;
pub mod auth;
pub mod avatar;
pub mod banner;
pub mod config;
pub mod connection;
pub mod context;
pub mod pool;
pub mod protocol;
pub mod reconnection;
pub mod rpc_client;
pub mod rpc_server;
pub mod session;
pub mod shell;
pub mod telnet;
pub mod webapp;
pub mod websocket;

