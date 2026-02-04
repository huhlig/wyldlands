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
use std::sync::Arc;
use tonic::transport::Server as TonicServer;
use tracing::{debug, info, instrument, Instrument, info_span};
use tracing_flame::FlameLayer;
use tracing_subscriber::{EnvFilter, Registry, fmt::time::ChronoUtc, prelude::*};
use wyldlands_common::proto::world_to_session_server::WorldToSessionServer;
use wyldlands_gateway::config::{Arguments, Configuration};
use wyldlands_gateway::context::ServerContext;
use wyldlands_gateway::grpc::{GatewayRpcServer, RpcClientManager};
use wyldlands_gateway::server::TermionixTelnetServer;

#[tokio::main]
#[instrument(name = "gateway_main")]
async fn main() {
    // Load arguments from the command line
    let arguments: Arguments = Parser::parse();

    let (flame_layer, _guard) = FlameLayer::with_file("gateway.folded").unwrap();

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

    info!("Tracing initialized");

    // Load environment variables from .env file if specified
    if let Some(ref env_file) = arguments.env_file {
        if std::path::Path::new(env_file).exists() {
            info!("Loading environment variables from file: {}", env_file);
            dotenv::from_filename(env_file).ok();
        }
    } else {
        // Try default .env file
        info!("Loading environment variables from default file");
        dotenv::dotenv().ok();
    }

    // Load configuration from a file with environment variable substitution
    let config: Configuration = Configuration::load(&arguments.config_file)
        .inspect_err(|err| {
            eprintln!(
                "Configuration load error for {}: {}",
                arguments.config_file, err
            )
        })
        .expect(&format!(
            "Unable to load configuration file {}",
            arguments.config_file
        ));

    debug!("Configuration loaded: {:?}", config);
    info!("Starting Wyldlands Gateway Server...");

    // Create an RPC client for server communication
    let _rpc_span = tracing::info_span!("rpc_client_setup").entered();
    let rpc_client = Arc::new(RpcClientManager::new(
        config.server.addr.as_str(),
        config.server.auth_key.as_str(),
        config.server.reconnect_interval,
        config.server.heartbeat_interval,
    ));

    // Start RPC client reconnection loop
    let rpc_client_reconnect = Arc::clone(&rpc_client);
    tokio::spawn(
        async move {
            rpc_client_reconnect.start_reconnection_loop().await;
        }
        .instrument(info_span!("rpc_reconnection_loop")),
    );

    // Start RPC client heartbeat loop
    // Note: We use a dummy session ID for the gateway-to-server heartbeat
    // In a real implementation, you might want to use a proper gateway identifier
    let rpc_client_heartbeat = Arc::clone(&rpc_client);
    let gateway_session_id = format!("gateway-{}", uuid::Uuid::new_v4());
    tokio::spawn(
        async move {
            rpc_client_heartbeat
                .start_heartbeat_loop(gateway_session_id)
                .await;
        }
        .instrument(info_span!("rpc_heartbeat_loop")),
    );

    // Create server context with session management
    // Default session timeout: 300 seconds (5 minutes)
    let context = ServerContext::new(300, rpc_client);

    // Spawn banner refresh task that runs after RPC connection
    let rpc_client_banner = Arc::clone(context.rpc_client());
    let banner_manager = Arc::clone(context.properties_manager());
    tokio::spawn(
        async move {
            loop {
                // Wait for an RPC client to be connected
                if rpc_client_banner.is_connected().await {
                    tracing::info!("RPC connected, refreshing banners from server");
                    let properties = vec![
                        wyldlands_common::gateway::GatewayProperty::BannerWelcome,
                        wyldlands_common::gateway::GatewayProperty::BannerMotd,
                        wyldlands_common::gateway::GatewayProperty::BannerLogin,
                        wyldlands_common::gateway::GatewayProperty::BannerLogout,
                        wyldlands_common::gateway::GatewayProperty::AdminHtml,
                        wyldlands_common::gateway::GatewayProperty::AdminCss,
                        wyldlands_common::gateway::GatewayProperty::AdminJs,
                        wyldlands_common::gateway::GatewayProperty::ClientHtml,
                        wyldlands_common::gateway::GatewayProperty::ClientCss,
                        wyldlands_common::gateway::GatewayProperty::ClientJs,
                    ];
                    if let Err(e) = banner_manager.refresh_cached_properties(&properties).await {
                        tracing::warn!("Failed to refresh banners: {}", e);
                    } else {
                        tracing::info!("Banners refreshed successfully");
                    }
                    // Wait 5 minutes before checking again (banner cache TTL)
                    tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                } else {
                    // Check connection status every 5 seconds
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
        .instrument(info_span!("banner_refresh_loop")),
    );

    // Spawn connection pool handler
    let pool = context.connection_pool().clone();
    tokio::spawn(
        async move {
            pool.run().await;
        }
        .instrument(info_span!("connection_pool_run")),
    );

    // Spawn session cleanup task (runs every 60 seconds)
    let session_manager = context.session_manager().clone();
    tokio::spawn(
        async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Err(e) = session_manager.cleanup_expired().await {
                    tracing::error!("Failed to cleanup expired sessions: {}", e);
                }
            }
        }
        .instrument(info_span!("session_cleanup_loop")),
    );

    // build our application with routes
    let webapp = wyldlands_gateway::server::router(&context);

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

    // Create gRPC server for receiving calls from world server (before telnet to avoid move)
    let grpc_addr = config
        .server
        .addr
        .into_inner()
        .to_addrs()
        .expect("Failed to resolve gRPC address")
        .next()
        .expect("No addresses resolved for gRPC server");
    let grpc_server = GatewayRpcServer::new(
        context.connection_pool().clone(),
        context.session_manager().clone(),
    );

    // Create termionix telnet server
    let telnet_config = config.telnet.clone().unwrap_or_default();
    let telnet_addr = telnet_config.addr.to_addr();
    let telnet_ip = telnet_config.addr.to_ip();
    let telnet_port = telnet_config.addr.to_port();

    info!(
        "Termionix Telnet Server will listen on {} ({}:{})",
        telnet_addr, telnet_ip, telnet_port,
    );

    let telnet_server = TermionixTelnetServer::new(context, telnet_config);

    info!("Starting gRPC server on {}", grpc_addr);

    // Spawn all services
    let webapp_handle = tokio::spawn(
        async move {
            axum::serve(http_listener, webapp)
                .await
                .expect("WebSocket server failed");
        }
        .instrument(info_span!("webapp_service")),
    );

    let telnet_handle = tokio::spawn(
        async move {
            if let Err(e) = telnet_server.run().await {
                tracing::error!("Telnet server error: {}", e);
            }
        }
        .instrument(info_span!("telnet_service")),
    );

    let grpc_handle = tokio::spawn(
        async move {
            if let Err(e) = TonicServer::builder()
                .add_service(WorldToSessionServer::new(grpc_server))
                .serve(grpc_addr)
                .await
            {
                tracing::error!("gRPC server error: {}", e);
            }
        }
        .instrument(info_span!("grpc_service")),
    );

    let _ = tokio::join!(webapp_handle, telnet_handle, grpc_handle);
}
