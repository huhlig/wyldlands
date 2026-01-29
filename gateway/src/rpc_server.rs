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
use std::sync::Arc;
use tonic::{Request, Response, Status};
use wyldlands_common::proto::{
    server_gateway_server::ServerGateway as GrpcServerGateway,
    DisconnectSessionRequest, Empty, EntityStateChangedRequest, SendOutputRequest,
    SendPromptRequest,
};

/// Gateway RPC server handler
///
/// Implements the ServerGateway trait to receive calls from the world server.
#[derive(Clone)]
pub struct GatewayRpcServer {
    /// Connection pool for routing messages to clients
    connection_pool: Arc<ConnectionPool>,
}

impl GatewayRpcServer {
    /// Create a new gateway RPC server
    pub fn new(connection_pool: Arc<ConnectionPool>) -> Self {
        Self { connection_pool }
    }
}

#[tonic::async_trait]
impl GrpcServerGateway for GatewayRpcServer {
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

        // Send each output message to the client
        for output in req.output {
            // Convert protobuf GameOutput to text
            let text = match output.output_type {
                Some(wyldlands_common::proto::game_output::OutputType::Text(text_output)) => {
                    text_output.content
                }
                Some(wyldlands_common::proto::game_output::OutputType::FormattedText(
                    formatted,
                )) => formatted.content,
                Some(wyldlands_common::proto::game_output::OutputType::System(system)) => {
                    system.message
                }
                Some(wyldlands_common::proto::game_output::OutputType::RoomDescription(room)) => {
                    format!(
                        "{}\r\n{}\r\nExits: {}\r\n",
                        room.name,
                        room.description,
                        room.exits.join(", ")
                    )
                }
                Some(wyldlands_common::proto::game_output::OutputType::Combat(combat)) => {
                    combat.message
                }
                Some(wyldlands_common::proto::game_output::OutputType::Structured(structured)) => {
                    format!("[Structured: {}]", structured.output_type)
                }
                None => {
                    tracing::warn!("Received empty GameOutput for session {}", req.session_id);
                    continue;
                }
            };

            // Send to connection pool (convert text to bytes)
            if let Err(e) = self.connection_pool.send(session_id, text.into_bytes()).await {
                tracing::warn!(
                    "Failed to send output to session {}: {}",
                    req.session_id,
                    e
                );
            }
        }

        Ok(Response::new(Empty {}))
    }

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
            tracing::warn!(
                "Failed to send prompt to session {}: {}",
                req.session_id,
                e
            );
        }

        Ok(Response::new(Empty {}))
    }

    /// Notify gateway of entity state changes
    async fn entity_state_changed(
        &self,
        request: Request<EntityStateChangedRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        tracing::debug!(
            "gRPC: Entity state changed for session {} (entity: {})",
            req.session_id,
            req.entity_id
        );

        // TODO: Implement state change notifications
        // For now, we just log it. In the future, this could update client UI
        // or send MSDP/GMCP updates for MUD clients

        Ok(Response::new(Empty {}))
    }

    /// Request to disconnect a session
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

        // Send disconnect message to client
        let disconnect_msg = format!("\r\nDisconnecting: {}\r\n", req.reason);
        if let Err(e) = self
            .connection_pool
            .send(session_id, disconnect_msg.into_bytes())
            .await
        {
            tracing::warn!(
                "Failed to send disconnect message to session {}: {}",
                req.session_id,
                e
            );
        }

        // Unregister the session from the connection pool
        if let Err(e) = self.connection_pool.unregister(session_id).await {
            tracing::warn!(
                "Failed to unregister session {}: {}",
                req.session_id,
                e
            );
        }

        Ok(Response::new(Empty {}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{manager::SessionManager, store::SessionStore};

    #[tokio::test]
    async fn test_gateway_rpc_server_creation() {
        // Create a mock database pool (in-memory for testing)
        let database = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect("postgres://localhost/test")
            .await;

        // Skip test if database is not available
        if database.is_err() {
            return;
        }

        let database = database.unwrap();
        let store = SessionStore::new(database);
        let session_manager = Arc::new(SessionManager::new(store, 300));
        let connection_pool = Arc::new(ConnectionPool::new(session_manager));

        let _server = GatewayRpcServer::new(connection_pool);
        // Server created successfully
    }
}


