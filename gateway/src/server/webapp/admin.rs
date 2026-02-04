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

//use crate::auth::CreateAccountRequest;
use crate::context::ServerContext;
use crate::session::ProtocolType;
use axum::body::Body;
use axum::response::Response;
use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use wyldlands_common::gateway::GatewayProperty;

/// Gateway statistics
#[derive(Debug, Serialize)]
pub struct SystemStatisticsResponse {
    pub gateway: GatewayStatisticsResponse,
    pub server: ServerStatisticsResponse,
}

#[derive(Debug, Serialize)]
pub struct GatewayStatisticsResponse {
    /// Total number of active sessions
    pub total_sessions: usize,

    /// Number of sessions by state
    pub sessions_by_state: HashMap<String, usize>,

    /// Number of connections by server
    pub connections_by_protocol: HashMap<String, usize>,

    /// Server uptime in seconds
    pub gateway_uptime: Duration,

    /// Memory usage (if available)
    pub memory_usage_mb: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ServerStatisticsResponse {
    // TODO: Collect Server Statistics
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

/// Account creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountRequest {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub email: Option<String>,
}

impl CreateAccountRequest {
    /// Validate the account creation request
    pub fn validate(&self) -> Result<(), String> {
        // Validate username
        if self.username.is_empty() {
            return Err("Username cannot be empty".to_string());
        }
        if self.username.len() < 3 {
            return Err("Username must be at least 3 characters".to_string());
        }
        if self.username.len() > 20 {
            return Err("Username must be at most 20 characters".to_string());
        }
        if !self
            .username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            return Err("Username can only contain letters, numbers, and underscores".to_string());
        }

        // Validate display name
        if self.display_name.is_empty() {
            return Err("Display name cannot be empty".to_string());
        }
        if self.display_name.len() > 50 {
            return Err("Display name must be at most 50 characters".to_string());
        }

        // Validate password
        if self.password.is_empty() {
            return Err("Password cannot be empty".to_string());
        }
        if self.password.len() < 6 {
            return Err("Password must be at least 6 characters".to_string());
        }
        if self.password.len() > 100 {
            return Err("Password must be at most 100 characters".to_string());
        }

        // Validate email if provided
        if let Some(email) = &self.email {
            if !email.is_empty() && !email.contains('@') {
                return Err("Invalid email address".to_string());
            }
        }

        Ok(())
    }
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
        .route("/", get(admin_html))
        .route("/admin.html", get(admin_html))
        .route("/admin.css", get(admin_css))
        .route("/admin.js", get(admin_js))
        .route("/stats", get(get_stats))
        .route("/sessions", get(list_sessions))
        .route("/sessions/{id}", get(get_session))
        .route("/command", post(execute_command))
        .route("/health", get(health_check))
        .route("/accounts/create", post(create_account))
        .route("/accounts/check/{username}", get(check_username))
}

/// Admin HTML handler
async fn admin_html(State(context): State<ServerContext>) -> impl IntoResponse {
    match context
        .properties_manager()
        .get_property(GatewayProperty::AdminHtml)
        .await
    {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Body::from(content))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("<h1>Error loading client page</h1>"))
            .unwrap(),
    }
}

/// Admin CSS handler
async fn admin_css(State(context): State<ServerContext>) -> impl IntoResponse {
    match context
        .properties_manager()
        .get_property(GatewayProperty::AdminCss)
        .await
    {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/css; charset=utf-8")
            .body(Body::from(content))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Error loading Admin CSS"))
            .unwrap(),
    }
}

/// Admin JS handler
async fn admin_js(State(context): State<ServerContext>) -> impl IntoResponse {
    match context
        .properties_manager()
        .get_property(GatewayProperty::AdminJs)
        .await
    {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/javascript; charset=utf-8")
            .body(Body::from(content))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Error loading Admin JS"))
            .unwrap(),
    }
}

/// Get gateway statistics
async fn get_stats(
    State(context): State<ServerContext>,
) -> Result<Json<SystemStatisticsResponse>, StatusCode> {
    let gateway = {
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

        // Get connections by sidechannel
        let telnet_count = connection_pool
            .count_by_protocol(ProtocolType::Telnet)
            .await;
        let websocket_count = connection_pool
            .count_by_protocol(ProtocolType::WebSocket)
            .await;

        let mut connections_by_protocol = HashMap::new();
        connections_by_protocol.insert("telnet".to_string(), telnet_count);
        connections_by_protocol.insert("websocket".to_string(), websocket_count);

        // Calculate uptime (would need to track start time in context)
        let gateway_uptime = context.gateway_uptime();

        // Get memory usage (platform-specific)
        let memory_usage_mb = None; // TODO: Implement memory tracking
        GatewayStatisticsResponse {
            total_sessions,
            sessions_by_state,
            connections_by_protocol,
            gateway_uptime,
            memory_usage_mb,
        }
    };
    let server = {
        let _server_stats: wyldlands_common::proto::ServerStatisticsResponse = context
            .rpc_client
            .fetch_server_statistics()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        ServerStatisticsResponse {
            // TODO: Populate Server Statistics
        }
    };
    Ok(Json(SystemStatisticsResponse { gateway, server }))
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
            id: s.session_id.to_string(),
            state: format!("{:?}", s.state),
            protocol: format!("{:?}", s.protocol),
            client_addr: s.client_addr.clone(),
            entity_id: s.account.as_ref().and_then(|a| Some(a.id.to_string())),
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
        id: session.session_id.to_string(),
        state: format!("{:?}", session.state),
        protocol: format!("{:?}", session.protocol),
        client_addr: session.client_addr.clone(),
        entity_id: session
            .account
            .as_ref()
            .and_then(|a| Some(a.id.to_string())),
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
                    message: format!("Failed to logout.txt session: {}", e),
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
    State(_context): State<ServerContext>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<Json<CreateAccountResponse>, StatusCode> {
    tracing::info!(
        "Account creation request for username: {}",
        request.username
    );

    // Validate the request
    if let Err(e) = request.validate() {
        tracing::warn!("Account creation validation failed: {}", e);
        return Ok(Json(CreateAccountResponse {
            success: false,
            message: e,
            account_id: None,
        }));
    }

    // TODO: Implement RPC call to create account on server
    tracing::warn!("Account creation not yet implemented via RPC");
    Ok(Json(CreateAccountResponse {
        success: false,
        message: "Account creation not yet implemented".to_string(),
        account_id: None,
    }))
}

/// Check if a username is available
async fn check_username(
    State(_context): State<ServerContext>,
    axum::extract::Path(username): axum::extract::Path<String>,
) -> Result<Json<UsernameCheckResponse>, StatusCode> {
    // TODO: Implement RPC call to check username availability
    tracing::warn!("Username check not yet implemented via RPC");
    Ok(Json(UsernameCheckResponse {
        available: true,
        username,
    }))
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
