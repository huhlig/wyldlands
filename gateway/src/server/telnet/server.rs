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
use crate::session::ProtocolType;
use std::collections::HashMap;
use std::sync::Arc;
use termionix_service::{
    ConnectionId, ServerConfig, ServerHandler, TelnetConnection, TelnetError, TelnetServer,
};
use termionix_terminal::TerminalEvent;
use tokio::sync::{RwLock, mpsc};
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

                        // Create an adapter and state handler
                        let mut adapter = TermionixAdapter::new(conn.clone(), id, event_rx);
                        let state_handler =
                            StateHandler::new(session_id, self.context.clone(), addr);

                        // Send the initial prompt in a separate task
                        tokio::spawn(async move {
                            if let Err(e) = state_handler.send_prompt(&mut adapter).await {
                                tracing::error!("Failed to send initial prompt: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Failed to register connection: {}", e);
                        let _ = conn.send("Failed to register connection\r\n").await;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to create session: {}", e);
                let _ = conn.send("Failed to create session\r\n").await;
            }
        }
    }

    async fn on_event(&self, id: ConnectionId, conn: &TelnetConnection, event: TerminalEvent) {
        // Get session ID and event sender for this connection
        let connection_info = self.connections.read().await.get(&id).cloned();

        if let Some((session_id, event_tx)) = connection_info {
            // Forward event to the adapter's event channel
            if event_tx.send(event.clone()).is_err() {
                tracing::warn!("Failed to forward event for connection {}", id);
                return;
            }

            // Check session state to determine input mode
            let session = self.context.session_manager().get_session(session_id).await;
            let is_editing = session
                .as_ref()
                .map(|s| s.state.is_editing())
                .unwrap_or(false);

            // Extract input based on event type and session state
            let input_opt = match &event {
                // In editing mode, process character-by-character
                TerminalEvent::CharacterData { character, .. } if is_editing => {
                    Some(character.to_string())
                }
                // In playing mode, process complete lines
                TerminalEvent::LineCompleted { line, .. } if !is_editing => Some(line.to_string()),
                // Don't process other combinations
                _ => None,
            };

            if let Some(input) = input_opt {
                tracing::debug!(
                    "Received input from connection {} (editing={}): '{}'",
                    id,
                    is_editing,
                    input
                );

                // Create adapter for this event
                let (_event_tx2, event_rx) = mpsc::unbounded_channel();
                let mut adapter = TermionixAdapter::new(conn.clone(), id, event_rx);
                let mut state_handler = StateHandler::new(
                    session_id,
                    self.context.clone(),
                    conn.peer_addr().to_string(),
                );

                // Process input
                match state_handler.process_input(&mut adapter, input).await {
                    Ok(()) => {
                        // Check if we've transitioned to authenticated state
                        if let Some(session) =
                            self.context.session_manager().get_session(session_id).await
                        {
                            if session.state.is_authenticated() {
                                // Send start command to server
                                tracing::info!(
                                    "Session {} authenticated, starting game",
                                    session_id
                                );

                                let rpc_client = self.context.rpc_client();
                                use wyldlands_common::proto::SessionToWorldClient;
                                if let Some(client) = rpc_client.session_client().await {
                                    let mut client: SessionToWorldClient = client;
                                    let request = wyldlands_common::proto::SendInputRequest {
                                        session_id: session_id.to_string(),
                                        command: "start".to_string(),
                                    };

                                    match client.send_input(request).await {
                                        Ok(response) => {
                                            let resp = response.into_inner();
                                            if resp.success {
                                                tracing::info!(
                                                    "Game started for session {}",
                                                    session_id
                                                );
                                            } else {
                                                let error_msg = resp
                                                    .error
                                                    .unwrap_or_else(|| "Unknown error".to_string());
                                                tracing::error!(
                                                    "Failed to start game: {}",
                                                    error_msg
                                                );
                                                let _ = conn
                                                    .send(&format!(
                                                        "Failed to start: {}\r\n",
                                                        error_msg
                                                    ))
                                                    .await;
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("RPC error starting game: {}", e);
                                            let _ =
                                                conn.send("Server communication error.\r\n").await;
                                        }
                                    }
                                }

                                // Generate reconnection token
                                match self.reconnection_manager.generate_token(session_id).await {
                                    Ok(token) => match token.encode() {
                                        Ok(encoded) => {
                                            let _ = conn
                                                .send(&format!(
                                                    "Your reconnection token: {}\r\n",
                                                    encoded
                                                ))
                                                .await;
                                            tracing::info!(
                                                "Generated reconnection token for session {}",
                                                session_id
                                            );
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                "Failed to encode reconnection token: {}",
                                                e
                                            );
                                        }
                                    },
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to generate reconnection token: {}",
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        // Send next prompt
                        if let Err(e) = state_handler.send_prompt(&mut adapter).await {
                            tracing::error!("Failed to send prompt: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error processing input: {}", e);
                        let _ = conn.send(&format!("Error: {}\r\n", e)).await;

                        // Send prompt again
                        if let Err(e) = state_handler.send_prompt(&mut adapter).await {
                            tracing::error!("Failed to send prompt: {}", e);
                        }
                    }
                }
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
        let connection_info = self.connections.write().await.remove(&id);

        if let Some((session_id, _)) = connection_info {
            // Display logout message
            if let Ok(disconnect_msg) = self
                .context
                .properties_manager()
                .get_property(GatewayProperty::BannerLogout)
                .await
            {
                let _ = conn.send(&disconnect_msg).await;
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
        let server_config = ServerConfig::new(bind_addr)
            .with_max_connections(1000)
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


