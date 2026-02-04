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

use clap::Parser;
use sqlx::Executor;
use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::instrument;
use tracing_flame::FlameLayer;
use tracing_subscriber::{EnvFilter, Registry, fmt::time::ChronoUtc, prelude::*};
use wyldlands_common::proto::{GatewayManagementServer, SessionToWorldServer};
use wyldlands_server::config::{Arguments, Configuration};
use wyldlands_server::listener::ServerRpcHandler;

#[tokio::main]
#[instrument(name = "server_main")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load arguments from the command line
    let arguments: Arguments = Parser::parse();

    let (flame_layer, _guard) = FlameLayer::with_file("server.folded")?;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .with_ansi(true)
        .with_timer(ChronoUtc::rfc_3339());

    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(fmt_layer)
        .with(flame_layer)
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
    let config: Configuration =
        Configuration::load(&arguments.config_file).expect("Unable to load configuration file");

    tracing::debug!("Configuration loaded: {:?}", config);
    tracing::info!("Starting Wyldlands World Server...");

    // Initialize the database connection pool
    tracing::info!("Connecting to Database at {}", &config.database.url);
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

    // Create a persistence manager (auto-save every 60 seconds)
    let persistence_manager = std::sync::Arc::new(
        wyldlands_server::persistence::PersistenceManager::new(database.clone(), 60),
    );
    tracing::info!("Persistence manager initialized");

    // Create world engine context
    let world_context = std::sync::Arc::new(wyldlands_server::ecs::context::WorldContext::new(
        persistence_manager.clone(),
    ));
    tracing::info!("World engine context initialized");

    // Load all persistent entities from the database
    tracing::info!("Loading world entities from database...");
    match world_context.load().await {
        Ok(count) => tracing::info!("Loaded {} entities from database", count),
        Err(e) => {
            tracing::error!("Failed to load world: {}", e);
            return Err(format!("Failed to load world: {}", e).into());
        }
    }

    // Start an auto-save task
    persistence_manager
        .clone()
        .start_auto_save_task(world_context.entities().clone());
    tracing::info!("Auto-save task started");

    // Get Server Address from configuration
    let listen_addr: SocketAddr = config.listener.addr.to_addr();

    tracing::info!("Starting gRPC server on {}", listen_addr);

    // Create the RPC handler with world context
    let handler = ServerRpcHandler::new(
        config.listener.auth_key.as_str(),
        world_context,
        &config.listener.gateway_addr.to_string(),
    );
    tracing::info!("Server RPC handler initialized with persistence");

    // Connect to gateway for sending messages back
    if let Err(e) = handler.connect_to_gateway().await {
        tracing::warn!("Failed to connect to gateway initially: {}. Will retry on first message.", e);
    }

    // Start gRPC server
    Server::builder()
        .add_service(GatewayManagementServer::new(handler.clone()))
        .add_service(SessionToWorldServer::new(handler))
        .serve(listen_addr)
        .await?;

    Ok(())
}
