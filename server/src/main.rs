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

use clap::Parser;
use futures::prelude::*;
use std::net::SocketAddr;
use tarpc::server::{BaseChannel, Channel};
use tarpc::tokio_serde::formats::Bincode;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;
use wyldlands_common::gateway::GatewayServer;
use wyldlands_server::config::{Arguments, Configuration};
use wyldlands_server::listener::ServerRpcHandler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let config: Configuration =
        Configuration::load(&arguments.config_file)
            .expect("Unable to load configuration file");

    tracing::debug!("Configuration loaded: {:?}", config);
    tracing::info!("Starting Wyldlands World Server...");

    // Initialize the database connection pool
    tracing::info!("Connecting to Database at {}", &config.database.url);
    let database = sqlx::postgres::PgPoolOptions::new()
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
    let world_context = std::sync::Arc::new(
        wyldlands_server::ecs::context::WorldContext::new(persistence_manager.clone()),
    );
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

    tracing::info!("Binding RPC server to {}", listen_addr);
    let listener = TcpListener::bind(listen_addr).await?;
    tracing::info!("RPC server listening on {}", listen_addr);

    // Create the RPC handler with world context
    let handler = ServerRpcHandler::new(config.listener.auth_key.as_str(), world_context);
    tracing::info!("Server RPC handler initialized with persistence");

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                tracing::info!("New RPC connection from {}", peer_addr);

                let handler = handler.clone();

                tokio::spawn(async move {
                    // Set up the transport
                    let transport = tarpc::serde_transport::new(
                        tokio_util::codec::LengthDelimitedCodec::builder()
                            .max_frame_length(16 * 1024 * 1024) // 16MB max frame
                            .new_framed(stream),
                        Bincode::default(),
                    );

                    // Create the server channel
                    let server = BaseChannel::with_defaults(transport);

                    // Serve requests - execute returns a stream of request handlers
                    // Each request handler is a future that needs to be spawned
                    server
                        .execute(handler.serve())
                        .for_each(|response| async move {
                            tokio::spawn(response);
                        })
                        .await;

                    tracing::info!("RPC connection closed: {}", peer_addr);
                });
            }
            Err(e) => {
                tracing::error!("Failed to accept connection: {}", e);
            }
        }
    }
}


