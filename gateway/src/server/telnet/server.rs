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

//! Termionix-based telnet server implementation

use crate::config::TelnetServerConfig;
use crate::context::ServerContext;
use crate::reconnection::ReconnectionManager;
use crate::server::telnet::adapter::TermionixAdapter;
use crate::server::telnet::handler::StateHandler;
use crate::server::{InputMode, ProtocolAdapter};
use crate::session::ProtocolType;
use std::collections::HashMap;
use std::sync::Arc;
use termionix_server::{
    ConnectionId, ServerConfig, ServerHandler, TelnetConnection, TelnetError, TelnetServer,
    TerminalEvent,
};
use tokio::sync::{RwLock, mpsc};
use tracing::{Instrument, info_span};
use uuid::Uuid;
use wyldlands_common::gateway::GatewayProperty;

/// Wyldlands handler for Termionix server
struct WyldlandsHandler {
    context: ServerContext,
    reconnection_manager: Arc<ReconnectionManager>,
    /// Map connection IDs to session IDs and event senders
    connections: Arc<RwLock<HashMap<ConnectionId, (Uuid, mpsc::UnboundedSender<TerminalEvent>)>>>,
}

impl WyldlandsHandler {
    fn new(context: ServerContext, reconnection_manager: Arc<ReconnectionManager>) -> Self {
        Self {
            context,
            reconnection_manager,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl ServerHandler for WyldlandsHandler {
    async fn on_connect(&self, id: ConnectionId, conn: &TelnetConnection) {
        tracing::info!("Termionix connection {} established", id);

        // Create session
        let addr = conn.peer_addr().to_string();

        match self
            .context
            .session_manager()
            .create_session(ProtocolType::Telnet, addr.clone())
            .await
        {
            Ok(session_id) => {
                tracing::info!("Created session {} for connection {}", session_id, id);

                // Register connection in pool
                match self
                    .context
                    .connection_pool()
                    .register(session_id, ProtocolType::Telnet)
                    .await
                {
                    Ok(_sender) => {
                        // Create event channel for this connection
                        let (event_tx, event_rx) = mpsc::unbounded_channel();

                        // Store connection mapping
                        self.connections
                            .write()
                            .await
                            .insert(id, (session_id, event_tx));

                        // Spawn a task to handle this connection's events
                        let context = self.context.clone();
                        let conn_clone = conn.clone();
                        let addr_clone = addr.clone();

                        tokio::spawn(
                            async move {
                                let mut adapter =
                                    TermionixAdapter::new(conn_clone.clone(), event_rx);
                                let mut state_handler =
                                    StateHandler::new(session_id, context.clone(), addr_clone);

                                // Update input mode immediately before sending prompt
                                if state_handler.is_editing() {
                                    adapter.set_input_mode(InputMode::Character);
                                } else {
                                    adapter.set_input_mode(InputMode::Line);
                                }

                                // Send initial prompt (adapter now flushes automatically)
                                if let Err(e) = state_handler.send_prompt(&mut adapter).await {
                                    tracing::error!("Failed to send initial prompt: {}", e);
                                }

                                // Event loop - optimized for hot path
                                use crate::server::ProtocolMessage;
                                while let Ok(msg_opt) = adapter.receive().await {
                                    if let Some(msg) = msg_opt {
                                        match msg {
                                            ProtocolMessage::Text(input) => {
                                                // Process input
                                                match state_handler
                                                    .process_input(&mut adapter, input)
                                                    .await
                                                {
                                                    Ok(()) => {
                                                        // Send next prompt
                                                        if let Err(e) = state_handler
                                                            .send_prompt(&mut adapter)
                                                            .await
                                                        {
                                                            tracing::error!(
                                                                "Failed to send prompt: {}",
                                                                e
                                                            );
                                                        }
                                                    }
                                                    Err(e) => {
                                                        tracing::error!(
                                                            "Error processing input: {}",
                                                            e
                                                        );
                                                        let _ = conn_clone
                                                            .send(
                                                                &format!("Error: {}\r\n", e),
                                                                true,
                                                            )
                                                            .await;

                                                        // Send prompt again
                                                        if let Err(e) = state_handler
                                                            .send_prompt(&mut adapter)
                                                            .await
                                                        {
                                                            tracing::error!(
                                                                "Failed to send prompt: {}",
                                                                e
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            ProtocolMessage::Disconnected => {
                                                tracing::info!(
                                                    "Adapter received disconnect for session {}",
                                                    session_id
                                                );
                                                break;
                                            }
                                            _ => {
                                                // Handle other sidechannel messages if needed
                                            }
                                        }
                                    }
                                }
                                tracing::info!(
                                    "Connection task for session {} finished",
                                    session_id
                                );
                            }
                            .instrument(info_span!("telnet_connection", session_id = %session_id)),
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to register connection: {}", e);
                        let _ = conn.send("Failed to register connection\r\n", true).await;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to create session: {}", e);
                let _ = conn.send("Failed to create session\r\n", true).await;
            }
        }
    }

    async fn on_event(&self, id: ConnectionId, _conn: &TelnetConnection, event: TerminalEvent) {
        // Get event sender for this connection
        let connection_info = self.connections.read().await.get(&id).cloned();

        if let Some((_session_id, event_tx)) = connection_info {
            // Forward event to the adapter's event channel
            if event_tx.send(event.clone()).is_err() {
                tracing::warn!("Failed to forward event for connection {}", id);
            }
        }
    }

    async fn on_error(&self, id: ConnectionId, _conn: &TelnetConnection, error: TelnetError) {
        tracing::error!("Error for connection {}: {}", id, error);
    }

    async fn on_timeout(&self, id: ConnectionId, _conn: &TelnetConnection) {
        tracing::warn!("Connection {} timed out", id);
    }

    async fn on_idle_timeout(&self, id: ConnectionId, _conn: &TelnetConnection) {
        tracing::info!("Connection {} idle timeout", id);
    }

    async fn on_disconnect(&self, id: ConnectionId, conn: &TelnetConnection) {
        tracing::info!("Connection {} disconnected", id);

        // Get session ID for this connection
        let connection_info = {
            let mut connections = self.connections.write().await;
            connections.remove(&id)
        };

        if let Some((session_id, _)) = connection_info {
            // Display logout message
            if let Ok(disconnect_msg) = self
                .context
                .properties_manager()
                .get_property(GatewayProperty::BannerLogout)
                .await
            {
                let _ = conn.send(&disconnect_msg, true).await;
            }

            // Generate reconnection token
            if let Ok(token) = self.reconnection_manager.generate_token(session_id).await {
                match token.encode() {
                    Ok(encoded) => {
                        tracing::info!(
                            "Reconnection token available for session {}: {}",
                            session_id,
                            encoded
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Failed to encode reconnection token: {}", e);
                    }
                }
            }

            // Unregister session from pool
            match self.context.connection_pool().unregister(session_id).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("Failed to unregister session {}: {}", session_id, e);
                }
            }

            // Mark the session as disconnected
            let _ = self
                .context
                .session_manager()
                .transition_session(session_id, crate::session::SessionState::Disconnected)
                .await;
        }
    }
}

/// Termionix-based telnet server
pub struct TermionixTelnetServer {
    context: ServerContext,
    config: TelnetServerConfig,
    reconnection_manager: Arc<ReconnectionManager>,
}

impl TermionixTelnetServer {
    /// Create a new termionix telnet server
    pub fn new(context: ServerContext, config: TelnetServerConfig) -> Self {
        let reconnection_manager = Arc::new(ReconnectionManager::new(
            context.clone(),
            3600, // 1 hour token TTL
        ));

        Self {
            context,
            config,
            reconnection_manager,
        }
    }

    /// Run the telnet server
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let bind_addr = self.config.addr.to_addr();
        tracing::info!("Starting Termionix telnet server on {}", bind_addr);

        // Create Termionix server config
        // Set read_timeout to 5 minutes to allow time for banner fetching and user input
        // The banners can take 20-30 seconds to fetch via RPC, so we need a longer timeout
        let server_config = ServerConfig::new(bind_addr)
            .with_max_connections(1000)
            .with_read_timeout(std::time::Duration::from_secs(300))
            .with_idle_timeout(std::time::Duration::from_secs(3600));

        // Create Termionix server
        let server = TelnetServer::new(server_config).await?;

        // Create handler
        let handler = Arc::new(WyldlandsHandler::new(
            self.context.clone(),
            self.reconnection_manager.clone(),
        ));

        // Start server
        server.start(handler).await?;

        tracing::info!("Termionix telnet server running");

        // Wait for shutdown signal
        tokio::signal::ctrl_c().await?;

        tracing::info!("Shutting down Termionix telnet server");
        server.shutdown().await?;

        Ok(())
    }
}
