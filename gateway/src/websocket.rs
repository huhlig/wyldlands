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

use crate::context::ServerContext;
use crate::protocol::websocket_adapter::WebSocketAdapter;
use crate::protocol::{ProtocolAdapter, ProtocolMessage};
use crate::reconnection::ReconnectionManager;
use crate::session::ProtocolType;
use axum::{
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, timeout};

/// WebSocket configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Enable compression
    pub enable_compression: bool,

    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,

    /// Client timeout in seconds
    pub client_timeout: u64,

    /// Enable reconnection support
    pub enable_reconnection: bool,

    /// Maximum message size in bytes
    pub max_message_size: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enable_compression: true,
            heartbeat_interval: 30,
            client_timeout: 60,
            enable_reconnection: true,
            max_message_size: 1024 * 1024, // 1MB
        }
    }
}

/// WebSocket handler with session management
pub async fn handler(ws: WebSocketUpgrade, State(context): State<ServerContext>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, context))
}

/// Handle WebSocket connection with full features
pub async fn handle_socket(socket: WebSocket, context: ServerContext) {
    let config = WebSocketConfig::default();

    // Create a reconnection manager
    let reconnection_manager = Arc::new(ReconnectionManager::new(
        context.clone(),
        3600, // 1 hour token TTL
    ));

    // Get the client address (would come from connection info in real implementation)
    let client_addr = "websocket-client".to_string();

    // Create session
    let session_id = match context
        .session_manager()
        .create_session(ProtocolType::WebSocket, client_addr)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to create WebSocket session: {}", e);
            return;
        }
    };

    tracing::info!("WebSocket session {} created", session_id);

    // Register connection in pool
    let _sender = match context
        .connection_pool()
        .register(session_id, ProtocolType::WebSocket)
        .await
    {
        Ok(sender) => sender,
        Err(e) => {
            tracing::error!("Failed to register WebSocket connection: {}", e);
            return;
        }
    };

    // Create protocol adapter
    let mut adapter = WebSocketAdapter::new(socket);

    // Enable compression if configured
    if config.enable_compression {
        adapter.enable_compression();
    }

    // Send a welcome message
    if let Err(e) = adapter.send_line("Welcome to Wyldlands MUD!").await {
        tracing::error!("Failed to send welcome message: {}", e);
        return;
    }

    // Generate and send a reconnection token if enabled
    if config.enable_reconnection {
        match reconnection_manager.generate_token(session_id).await {
            Ok(token) => match token.encode() {
                Ok(encoded) => {
                    let token_msg = format!("Your reconnection token: {}", encoded);
                    if let Err(e) = adapter.send_line(&token_msg).await {
                        tracing::error!("Failed to send reconnection token: {}", e);
                    } else {
                        tracing::info!("Generated reconnection token for session {}", session_id);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to encode reconnection token: {}", e);
                }
            },
            Err(e) => {
                tracing::warn!(
                    "Failed to generate reconnection token for session {}: {}",
                    session_id,
                    e
                );
            }
        }
    }

    // Start heartbeat task
    let heartbeat_handle = if config.heartbeat_interval > 0 {
        let session_manager = context.session_manager().clone();
        let sid = session_id;
        let interval_duration = Duration::from_secs(config.heartbeat_interval);

        Some(tokio::spawn(async move {
            let mut heartbeat = interval(interval_duration);
            loop {
                heartbeat.tick().await;

                // Touch session to keep it alive
                if let Err(e) = session_manager.touch_session(sid).await {
                    tracing::error!("Heartbeat failed for session {}: {}", sid, e);
                    break;
                }

                tracing::debug!("Heartbeat for session {}", sid);
            }
        }))
    } else {
        None
    };

    // Main message loop with timeout
    let timeout_duration = Duration::from_secs(config.client_timeout);

    loop {
        // Receive message with timeout
        let result = timeout(timeout_duration, adapter.receive()).await;

        match result {
            Ok(Ok(Some(msg))) => {
                match msg {
                    ProtocolMessage::Text(text) => {
                        tracing::debug!("Received text from session {}: {}", session_id, text);

                        // Echo back for now (will be replaced with game logic)
                        if let Err(e) = adapter.send_line(&format!("Echo: {}", text)).await {
                            tracing::error!("Failed to send response: {}", e);
                            break;
                        }

                        // Touch session on activity
                        let _ = context.session_manager().touch_session(session_id).await;
                    }
                    ProtocolMessage::Binary(data) => {
                        tracing::debug!(
                            "Received binary data from session {}: {} bytes",
                            session_id,
                            data.len()
                        );

                        // Handle binary data (could be compressed or protocol-specific)
                        if data.len() > config.max_message_size {
                            tracing::warn!("Message too large from session {}", session_id);
                            break;
                        }
                    }
                    ProtocolMessage::Ping => {
                        tracing::debug!("Ping from session {}", session_id);
                        // Pong is handled automatically by the adapter
                    }
                    ProtocolMessage::Pong => {
                        tracing::debug!("Pong from session {}", session_id);
                    }
                    ProtocolMessage::Disconnected => {
                        tracing::info!("Session {} disconnected", session_id);

                        // Generate a reconnection token on disconnect if enabled
                        if config.enable_reconnection {
                            if let Ok(token) = reconnection_manager.generate_token(session_id).await
                            {
                                match token.encode() {
                                    Ok(encoded) => {
                                        tracing::info!(
                                            "Reconnection token available for session {}: {}",
                                            session_id,
                                            encoded
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to encode reconnection token: {}",
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        break;
                    }
                    _ => {
                        tracing::debug!("Other message type from session {}", session_id);
                    }
                }
            }
            Ok(Ok(None)) => {
                // No message, continue
                continue;
            }
            Ok(Err(e)) => {
                tracing::error!("Protocol error for session {}: {}", session_id, e);
                break;
            }
            Err(_) => {
                tracing::warn!("Client timeout for session {}", session_id);
                break;
            }
        }
    }

    // Cleanup
    if let Some(handle) = heartbeat_handle {
        handle.abort();
    }

    // Close adapter
    let _ = adapter.close().await;

    // Unregister connection
    if let Err(e) = context.connection_pool().unregister(session_id).await {
        tracing::error!("Failed to unregister session {}: {}", session_id, e);
    }

    tracing::info!("WebSocket session {} closed", session_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        assert!(config.enable_compression);
        assert_eq!(config.heartbeat_interval, 30);
        assert_eq!(config.client_timeout, 60);
        assert!(config.enable_reconnection);
        assert_eq!(config.max_message_size, 1024 * 1024);
    }

    #[test]
    fn test_websocket_config_custom() {
        let config = WebSocketConfig {
            enable_compression: false,
            heartbeat_interval: 60,
            client_timeout: 120,
            enable_reconnection: false,
            max_message_size: 512 * 1024,
        };

        assert!(!config.enable_compression);
        assert_eq!(config.heartbeat_interval, 60);
        assert_eq!(config.client_timeout, 120);
        assert!(!config.enable_reconnection);
        assert_eq!(config.max_message_size, 512 * 1024);
    }
}
