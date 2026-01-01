//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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

mod admin;
mod auth;
mod avatar;
mod banner;
mod connection;
mod context;
mod pool;
mod protocol;
mod reconnection;
mod rpc_client;
mod session;
mod shell;
mod telnet;
mod webapp;
mod websocket;

use crate::context::ServerContext;
use crate::rpc_client::RpcClientManager;
use axum::{Router, routing::get};
use clap::Parser;
use sqlx::Executor;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;
use wyldlands_gateway::config::{Arguments, Configuration};

#[tokio::main]
async fn main() {
    // Load arguments from the command line
    let arguments: Arguments = Parser::parse();

    // Initialize tracing/logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .with_ansi(true)
        .init();

    // Load environment variables from .env file if specified
    if let Some(ref env_file) = arguments.env_file {
        if std::path::Path::new(env_file).exists() {
            tracing::debug!("Loading environment variables from file: {}", env_file);
            dotenv::from_filename(env_file).ok();
        }
    } else {
        // Try default .env file
        tracing::debug!("Loading environment variables from default file");
        dotenv::dotenv().ok();
    }

    // Load configuration from a file with environment variable substitution
    let config: Configuration = Configuration::load(&arguments.config_file)
        .inspect_err(|err| eprintln!("Configuration load error: {}", err))
        .expect("Unable to load configuration file");

    debug!("Configuration loaded: {:?}", config);
    info!("Starting Wyldlands Gateway Server...");

    // Initialize the database connection pool
    info!("Connecting to Database at {}", &config.database.url);
    let database = sqlx::postgres::PgPoolOptions::new()
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                // Set the search path for this specific connection
                conn.execute("SET search_path = wyldlands, public;").await?;
                Ok(())
            })
        })
        .max_connections(5)
        .connect(&config.database.url)
        .await
        .expect("Failed to connect to database");

    // Create RPC client for server communication
    let rpc_client = Arc::new(RpcClientManager::new(
        config.server.addr.as_str(),
        config.server.auth_key.as_str(),
        config.server.reconnect_interval,
        config.server.heartbeat_interval,
    ));

    // Start RPC client reconnection loop
    let rpc_client_reconnect = Arc::clone(&rpc_client);
    tokio::spawn(async move {
        rpc_client_reconnect.start_reconnection_loop().await;
    });

    // Start RPC client heartbeat loop
    // Note: We use a dummy session ID for the gateway-to-server heartbeat
    // In a real implementation, you might want to use a proper gateway identifier
    let rpc_client_heartbeat = Arc::clone(&rpc_client);
    let gateway_session_id = format!("gateway-{}", uuid::Uuid::new_v4());
    tokio::spawn(async move {
        rpc_client_heartbeat
            .start_heartbeat_loop(gateway_session_id)
            .await;
    });

    // Create server context with session management
    // Default session timeout: 300 seconds (5 minutes)
    let context = ServerContext::new(database, 300, rpc_client);

    // Spawn connection pool handler
    let pool = context.connection_pool().clone();
    tokio::spawn(async move {
        pool.run().await;
    });

    // Spawn session cleanup task (runs every 60 seconds)
    let session_manager = context.session_manager().clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = session_manager.cleanup_expired().await {
                tracing::error!("Failed to cleanup expired sessions: {}", e);
            }
        }
    });

    // build our application with routes
    let webapp = Router::new()
        .route("/client.html", get(webapp::client_page))
        .route("/client.css", get(webapp::client_css))
        .route("/client.js", get(webapp::client_js))
        .route("/websocket", get(websocket::handler))
        .nest("/admin", admin::create_admin_router())
        .with_state(context.clone());

    // Get websocket config or use defaults
    let websocket_config = config.websocket.unwrap_or_default();
    let http_listener = tokio::net::TcpListener::bind(websocket_config.addr.to_addr())
        .await
        .expect("Unable to bind to the websocket port");

    info!(
        "WebSocket Server listening on {} ({}:{})",
        websocket_config.addr,
        websocket_config.addr.to_ip(),
        websocket_config.addr.to_port()
    );

    // Create telnet server
    let telnet_config = telnet::TelnetConfig::default();
    let telnet_server = telnet::TelnetServer::new(context, telnet_config);

    // Get telnet config or use defaults
    let telnet_bind_config = config.telnet.unwrap_or_default();
    let telnet_listener = tokio::net::TcpListener::bind(telnet_bind_config.addr.to_addr())
        .await
        .expect("Unable to bind to telnet port");

    info!(
        "Telnet Server listening on {} ({}:{})",
        telnet_bind_config.addr,
        telnet_bind_config.addr.to_ip(),
        telnet_bind_config.addr.to_port(),
    );

    // Spawn both services
    let webapp_handle = tokio::spawn(async move {
        axum::serve(http_listener, webapp)
            .await
            .expect("WebSocket server failed");
    });

    let telnet_handle = tokio::spawn(async move {
        if let Err(e) = telnet_server.run(telnet_listener).await {
            tracing::error!("Telnet server error: {}", e);
        }
    });

    let _ = tokio::join!(webapp_handle, telnet_handle);
}
