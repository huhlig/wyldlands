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

//! Admin REST API for gateway management and statistics

use crate::auth::CreateAccountRequest;
use crate::context::ServerContext;
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Gateway statistics
#[derive(Debug, Serialize)]
pub struct GatewayStats {
    /// Total number of active sessions
    pub total_sessions: usize,

    /// Number of sessions by state
    pub sessions_by_state: HashMap<String, usize>,

    /// Number of connections by protocol
    pub connections_by_protocol: HashMap<String, usize>,

    /// Server uptime in seconds
    pub uptime_seconds: u64,

    /// Memory usage (if available)
    pub memory_usage_mb: Option<f64>,
}

/// Admin command request
#[derive(Debug, Serialize, Deserialize)]
pub struct AdminCommand {
    /// Command to execute
    pub command: String,

    /// Optional parameters
    pub params: Option<HashMap<String, String>>,
}

/// Admin command response
#[derive(Debug, Serialize)]
pub struct AdminCommandResponse {
    /// Whether the command succeeded
    pub success: bool,

    /// Response message
    pub message: String,

    /// Optional result data
    pub data: Option<serde_json::Value>,
}

/// Query parameters for session listing
#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    /// Filter by state
    pub state: Option<String>,

    /// Limit number of results
    pub limit: Option<usize>,
}

/// Session information for admin view
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub state: String,
    pub protocol: String,
    pub client_addr: String,
    pub entity_id: Option<String>,
    pub created_at: String,
    pub last_activity: String,
}

/// Account creation response
#[derive(Debug, Serialize)]
pub struct CreateAccountResponse {
    pub success: bool,
    pub message: String,
    pub account_id: Option<String>,
}

/// Username availability check response
#[derive(Debug, Serialize)]
pub struct UsernameCheckResponse {
    pub available: bool,
    pub username: String,
}

/// Create admin API router
pub fn create_admin_router() -> Router<ServerContext> {
    Router::new()
        .route("/stats", get(get_stats))
        .route("/sessions", get(list_sessions))
        .route("/sessions/{id}", get(get_session))
        .route("/command", post(execute_command))
        .route("/health", get(health_check))
        .route("/accounts/create", post(create_account))
        .route("/accounts/check/{username}", get(check_username))
}

/// Get gateway statistics
async fn get_stats(State(context): State<ServerContext>) -> Result<Json<GatewayStats>, StatusCode> {
    let session_manager = context.session_manager();
    let connection_pool = context.connection_pool();

    // Get total sessions
    let total_sessions = session_manager.session_count().await;

    // Get active sessions to count by state
    let active_sessions = session_manager.get_active_sessions().await;
    let mut sessions_by_state = HashMap::new();
    for session in &active_sessions {
        let state = format!("{:?}", session.state);
        *sessions_by_state.entry(state).or_insert(0) += 1;
    }

    // Get connections by protocol
    let telnet_count = connection_pool
        .count_by_protocol(crate::session::ProtocolType::Telnet)
        .await;
    let websocket_count = connection_pool
        .count_by_protocol(crate::session::ProtocolType::WebSocket)
        .await;

    let mut connections_by_protocol = HashMap::new();
    connections_by_protocol.insert("telnet".to_string(), telnet_count);
    connections_by_protocol.insert("websocket".to_string(), websocket_count);

    // Calculate uptime (would need to track start time in context)
    let uptime_seconds = 0; // TODO: Track actual uptime

    // Get memory usage (platform-specific)
    let memory_usage_mb = None; // TODO: Implement memory tracking

    Ok(Json(GatewayStats {
        total_sessions,
        sessions_by_state,
        connections_by_protocol,
        uptime_seconds,
        memory_usage_mb,
    }))
}

/// List sessions with optional filtering
async fn list_sessions(
    State(context): State<ServerContext>,
    Query(query): Query<SessionQuery>,
) -> Result<Json<Vec<SessionInfo>>, StatusCode> {
    let session_manager = context.session_manager();

    // Get all active sessions
    let sessions = session_manager.get_active_sessions().await;

    // Convert to SessionInfo and apply filters
    let mut session_infos: Vec<SessionInfo> = sessions
        .into_iter()
        .filter(|s| {
            if let Some(ref state_filter) = query.state {
                format!("{:?}", s.state).to_lowercase() == state_filter.to_lowercase()
            } else {
                true
            }
        })
        .map(|s| SessionInfo {
            id: s.id.to_string(),
            state: format!("{:?}", s.state),
            protocol: format!("{:?}", s.protocol),
            client_addr: s.client_addr.clone(),
            entity_id: s.entity_id.map(|id| id.to_string()),
            created_at: s.created_at.to_rfc3339(),
            last_activity: s.last_activity.to_rfc3339(),
        })
        .collect();

    // Apply limit
    if let Some(limit) = query.limit {
        session_infos.truncate(limit);
    }

    Ok(Json(session_infos))
}

/// Get specific session details
async fn get_session(
    State(context): State<ServerContext>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<SessionInfo>, StatusCode> {
    let session_manager = context.session_manager();

    // Parse session ID
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Get session
    let session = session_manager
        .get_session(uuid)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(SessionInfo {
        id: session.id.to_string(),
        state: format!("{:?}", session.state),
        protocol: format!("{:?}", session.protocol),
        client_addr: session.client_addr.clone(),
        entity_id: session.entity_id.map(|id| id.to_string()),
        created_at: session.created_at.to_rfc3339(),
        last_activity: session.last_activity.to_rfc3339(),
    }))
}

/// Execute admin command
async fn execute_command(
    State(context): State<ServerContext>,
    Json(command): Json<AdminCommand>,
) -> Result<Json<AdminCommandResponse>, StatusCode> {
    tracing::info!("Admin command received: {}", command.command);

    let response = match command.command.as_str() {
        "cleanup_sessions" => match context.session_manager().cleanup_expired().await {
            Ok(count) => AdminCommandResponse {
                success: true,
                message: format!("Cleaned up {} expired sessions", count),
                data: Some(serde_json::json!({ "cleaned": count })),
            },
            Err(e) => AdminCommandResponse {
                success: false,
                message: format!("Failed to cleanup sessions: {}", e),
                data: None,
            },
        },
        "broadcast" => {
            let message = command
                .params
                .as_ref()
                .and_then(|p| p.get("message"))
                .ok_or(StatusCode::BAD_REQUEST)?;

            match context
                .connection_pool()
                .broadcast(message.clone().into_bytes())
                .await
            {
                Ok(_) => AdminCommandResponse {
                    success: true,
                    message: "Broadcast sent to all connections".to_string(),
                    data: None,
                },
                Err(e) => AdminCommandResponse {
                    success: false,
                    message: format!("Failed to broadcast: {}", e),
                    data: None,
                },
            }
        }
        "disconnect_session" => {
            let session_id = command
                .params
                .as_ref()
                .and_then(|p| p.get("session_id"))
                .ok_or(StatusCode::BAD_REQUEST)?;

            let uuid = uuid::Uuid::parse_str(session_id).map_err(|_| StatusCode::BAD_REQUEST)?;

            match context.session_manager().remove_session(uuid).await {
                Ok(_) => AdminCommandResponse {
                    success: true,
                    message: format!("Session {} disconnected", session_id),
                    data: None,
                },
                Err(e) => AdminCommandResponse {
                    success: false,
                    message: format!("Failed to disconnect session: {}", e),
                    data: None,
                },
            }
        }
        _ => AdminCommandResponse {
            success: false,
            message: format!("Unknown command: {}", command.command),
            data: None,
        },
    };

    Ok(Json(response))
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Create a new account
async fn create_account(
    State(context): State<ServerContext>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<Json<CreateAccountResponse>, StatusCode> {
    tracing::info!("Account creation request for username: {}", request.username);
    
    match context.auth_manager().create_account(request).await {
        Ok(account) => {
            tracing::info!("Account created successfully: {}", account.id);
            Ok(Json(CreateAccountResponse {
                success: true,
                message: "Account created successfully".to_string(),
                account_id: Some(account.id.to_string()),
            }))
        }
        Err(e) => {
            tracing::warn!("Account creation failed: {}", e);
            Ok(Json(CreateAccountResponse {
                success: false,
                message: e,
                account_id: None,
            }))
        }
    }
}

/// Check if a username is available
async fn check_username(
    State(context): State<ServerContext>,
    axum::extract::Path(username): axum::extract::Path<String>,
) -> Result<Json<UsernameCheckResponse>, StatusCode> {
    match context.auth_manager().is_username_available(&username).await {
        Ok(available) => Ok(Json(UsernameCheckResponse {
            available,
            username,
        })),
        Err(e) => {
            tracing::error!("Failed to check username availability: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_command_serialization() {
        let cmd = AdminCommand {
            command: "test".to_string(),
            params: Some(HashMap::from([("key".to_string(), "value".to_string())])),
        };

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_admin_response_serialization() {
        let response = AdminCommandResponse {
            success: true,
            message: "Success".to_string(),
            data: Some(serde_json::json!({"count": 5})),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Success"));
    }
}


