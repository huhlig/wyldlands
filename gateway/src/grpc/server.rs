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

//! Gateway RPC server for receiving calls from the world server

use crate::pool::ConnectionPool;
use crate::session::manager::SessionManager;
use crate::session::{AuthenticatedState, SessionState};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use wyldlands_common::proto::{
    DisconnectSessionRequest, EditRequest, Empty, SendOutputRequest, SendPromptRequest,
    WorldToSession,
};

/// Gateway RPC server handler
///
/// Implements the ServerGateway trait to receive calls from the world server.
#[derive(Clone)]
pub struct GatewayRpcServer {
    /// Connection pool for routing messages to clients
    connection_pool: Arc<ConnectionPool>,

    /// Session manager for updating session state
    session_manager: Arc<SessionManager>,
}

impl GatewayRpcServer {
    /// Create a new gateway RPC server
    pub fn new(connection_pool: Arc<ConnectionPool>, session_manager: Arc<SessionManager>) -> Self {
        Self {
            connection_pool,
            session_manager,
        }
    }

    /// Send structured output to a client based on their capabilities
    ///
    /// Routes structured data to the appropriate protocol:
    /// - GMCP for telnet clients that support it
    /// - MSDP for telnet clients that support it
    /// - WebSocket JSON for WebSocket clients
    /// - Plain text fallback for clients without side channel support
    async fn send_structured_output(
        &self,
        session_id: uuid::Uuid,
        structured: &wyldlands_common::proto::StructuredOutput,
        capabilities: &crate::session::SideChannelCapabilities,
    ) -> Result<(), String> {
        // Try GMCP first (preferred for modern clients)
        if capabilities.gmcp {
            match crate::protocol::gmcp::encode_structured_output(structured) {
                Ok(data) => {
                    tracing::debug!(
                        "Sending structured output via GMCP to session {}",
                        session_id
                    );
                    return self.connection_pool.send(session_id, data).await;
                }
                Err(e) => {
                    tracing::warn!("Failed to encode GMCP for session {}: {}", session_id, e);
                }
            }
        }

        // Try MSDP next
        if capabilities.msdp {
            match crate::protocol::msdp::encode_structured_output(structured) {
                Ok(data) => {
                    tracing::debug!(
                        "Sending structured output via MSDP to session {}",
                        session_id
                    );
                    return self.connection_pool.send(session_id, data).await;
                }
                Err(e) => {
                    tracing::warn!("Failed to encode MSDP for session {}: {}", session_id, e);
                }
            }
        }

        // Try WebSocket JSON
        if capabilities.websocket_json {
            match crate::protocol::json::encode_structured_output(structured) {
                Ok(json_str) => {
                    tracing::debug!(
                        "Sending structured output via WebSocket JSON to session {}",
                        session_id
                    );
                    return self
                        .connection_pool
                        .send(session_id, json_str.into_bytes())
                        .await;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to encode WebSocket JSON for session {}: {}",
                        session_id,
                        e
                    );
                }
            }
        }

        // Fallback to plain text representation
        tracing::debug!(
            "No side channel available, sending structured output as plain text to session {}",
            session_id
        );
        let fallback_text = format!("[{}]\n", structured.output_type);
        self.connection_pool
            .send(session_id, fallback_text.into_bytes())
            .await
    }
}

#[tonic::async_trait]
impl WorldToSession for GatewayRpcServer {
    /// Send a prompt to a client session
    async fn send_prompt(
        &self,
        request: Request<SendPromptRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        tracing::debug!("gRPC: Sending prompt to session {}", req.session_id);

        // Convert session_id string to UUID
        let session_id = uuid::Uuid::parse_str(&req.session_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid session ID: {}", e)))?;

        // Send prompt to connection pool (convert to bytes)
        if let Err(e) = self
            .connection_pool
            .send(session_id, req.prompt.into_bytes())
            .await
        {
            tracing::warn!("Failed to send prompt to session {}: {}", req.session_id, e);
        }

        Ok(Response::new(Empty {}))
    }

    /// Send output to a client session
    async fn send_output(
        &self,
        request: Request<SendOutputRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        tracing::debug!(
            "gRPC: Sending {} output messages to session {}",
            req.output.len(),
            req.session_id
        );

        // Convert session_id string to UUID
        let session_id = uuid::Uuid::parse_str(&req.session_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid session ID: {}", e)))?;

        // Get session to check capabilities
        let session = self
            .session_manager
            .get_session(session_id)
            .await
            .ok_or_else(|| Status::not_found(format!("Session {} not found", session_id)))?;

        let side_channel_caps = &session.metadata.side_channel_capabilities;

        // Send each output message to the client
        for output in req.output {
            match output.output_type {
                Some(wyldlands_common::proto::game_output::OutputType::Text(text_output)) => {
                    // Plain text - send directly
                    if let Err(e) = self
                        .connection_pool
                        .send(session_id, text_output.content.into_bytes())
                        .await
                    {
                        tracing::warn!(
                            "Failed to send text output to session {}: {}",
                            req.session_id,
                            e
                        );
                    }
                }
                Some(wyldlands_common::proto::game_output::OutputType::FormattedText(
                    formatted,
                )) => {
                    // Formatted text - send content (formatting handled by client)
                    if let Err(e) = self
                        .connection_pool
                        .send(session_id, formatted.content.into_bytes())
                        .await
                    {
                        tracing::warn!(
                            "Failed to send formatted output to session {}: {}",
                            req.session_id,
                            e
                        );
                    }
                }
                Some(wyldlands_common::proto::game_output::OutputType::Structured(structured)) => {
                    // Structured data - route based on client capabilities
                    if let Err(e) = self
                        .send_structured_output(session_id, &structured, side_channel_caps)
                        .await
                    {
                        tracing::warn!(
                            "Failed to send structured output to session {}: {}",
                            req.session_id,
                            e
                        );
                    }
                }
                None => {
                    tracing::warn!("Received empty GameOutput for session {}", req.session_id);
                    continue;
                }
            }
        }

        Ok(Response::new(Empty {}))
    }

    /// Begin editing mode for a session
    async fn begin_editing(
        &self,
        request: Request<EditRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        tracing::info!(
            "gRPC: Begin editing for session {} (title: {})",
            req.session_id,
            req.title
        );

        // Convert session_id string to UUID
        let session_id = uuid::Uuid::parse_str(&req.session_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid session ID: {}", e)))?;

        // Get the session and update its state to Editing mode
        let mut session = self
            .session_manager
            .get_session(session_id)
            .await
            .ok_or_else(|| Status::not_found(format!("Session {} not found", req.session_id)))?;

        session.state = SessionState::Authenticated(AuthenticatedState::Editing {
            title: req.title.clone(),
            content: req.content.clone(),
        });

        self.session_manager
            .update_session(session)
            .await
            .map_err(|e| Status::internal(format!("Failed to update session state: {}", e)))?;

        // Send editing instructions to the client
        let instructions = format!(
            "\r\n=== Editing: {} ===\r\n\
            Use arrow keys to navigate, type to edit.\r\n\
            Press Ctrl+S to save, Escape to cancel.\r\n\
            \r\n{}\r\n",
            req.title, req.content
        );

        if let Err(e) = self
            .connection_pool
            .send(session_id, instructions.into_bytes())
            .await
        {
            tracing::warn!(
                "Failed to send editing instructions to session {}: {}",
                req.session_id,
                e
            );
        }

        tracing::info!("Session {} entered editing mode", req.session_id);

        Ok(Response::new(Empty {}))
    }

    /// Request to logout.txt a session
    async fn disconnect_session(
        &self,
        request: Request<DisconnectSessionRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        tracing::info!(
            "gRPC: Disconnect request for session {} (reason: {})",
            req.session_id,
            req.reason
        );

        // Convert session_id string to UUID
        let session_id = uuid::Uuid::parse_str(&req.session_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid session ID: {}", e)))?;

        // Send logout.txt message to client
        let disconnect_msg = format!("\r\nDisconnecting: {}\r\n", req.reason);
        if let Err(e) = self
            .connection_pool
            .send(session_id, disconnect_msg.into_bytes())
            .await
        {
            tracing::warn!(
                "Failed to send logout.txt message to session {}: {}",
                req.session_id,
                e
            );
        }

        // Unregister the session from the connection pool
        if let Err(e) = self.connection_pool.unregister(session_id).await {
            tracing::warn!("Failed to unregister session {}: {}", req.session_id, e);
        }

        Ok(Response::new(Empty {}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::manager::SessionManager;

    #[tokio::test]
    async fn test_gateway_rpc_server_creation() {
        let session_manager = Arc::new(SessionManager::new(300));
        let connection_pool = Arc::new(ConnectionPool::new(session_manager.clone()));

        let _server = GatewayRpcServer::new(connection_pool, session_manager);
        // Server created successfully
    }
}
