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

//! RPC client manager with automatic reconnection and command queuing

use metrics::{counter, gauge, histogram};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tonic::transport::Channel;
use tracing::{Level, event};
use wyldlands_common::proto::{AuthenticateGatewayRequest, AuthenticateSessionRequest, CheckUsernameRequest, CreateAccountRequest, GatewayHeartbeatRequest, GatewayManagementClient, SendInputRequest, ServerStatisticsRequest, SessionToWorldClient};

/// RPC client state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// Not connected
    Disconnected,
    /// Attempting to connect
    Connecting,
    /// Connected and ready
    Connected,
    /// Connection failed
    Failed,
}

/// Queued command to be executed when connection is restored
#[derive(Debug, Clone)]
pub struct QueuedCommand {
    /// Session ID for the command
    pub session_id: String,

    /// Command string
    pub command: String,

    /// Timestamp when command was queued
    pub queued_at: std::time::Instant,
}

/// Command queue statistics
#[derive(Debug, Clone)]
pub struct QueueStats {
    /// Number of commands currently queued
    pub queued_count: usize,

    /// Number of commands processed since last reconnection
    pub processed_count: usize,

    /// Number of commands dropped due to queue overflow
    pub dropped_count: usize,

    /// Maximum queue size
    pub max_queue_size: usize,
}

/// RPC client manager with automatic reconnection and command queuing
pub struct RpcClientManager {
    /// Server address
    server_addr: String,

    /// Authentication key for gateway-to-server communication
    auth_key: String,

    /// Gateway management client (if connected)
    gateway_client: Arc<RwLock<Option<GatewayManagementClient>>>,

    /// Session-to-server client (if connected)
    session_client: Arc<RwLock<Option<SessionToWorldClient>>>,

    /// Connection state
    state: Arc<RwLock<ClientState>>,

    /// Reconnection interval in seconds
    reconnect_interval: Duration,

    /// Heartbeat interval in seconds
    heartbeat_interval: Duration,

    /// Maximum reconnection attempts (0 = infinite)
    max_attempts: usize,

    /// Command queue for when disconnected
    command_queue: Arc<RwLock<VecDeque<QueuedCommand>>>,

    /// Maximum queue size (0 = unlimited)
    max_queue_size: usize,

    /// Number of commands dropped due to queue overflow
    dropped_count: Arc<RwLock<usize>>,

    /// Number of commands processed since last reconnection
    processed_count: Arc<RwLock<usize>>,
}

impl RpcClientManager {
    /// Create a new RPC client manager
    pub fn new(
        server_addr: &str,
        auth_key: &str,
        reconnect_interval_secs: u64,
        heartbeat_interval_secs: u64,
    ) -> Self {
        Self::with_queue_size(
            server_addr,
            auth_key,
            reconnect_interval_secs,
            heartbeat_interval_secs,
            1000,
        )
    }

    /// Create a new RPC client manager with custom queue size
    pub fn with_queue_size(
        server_addr: &str,
        auth_key: &str,
        reconnect_interval_secs: u64,
        heartbeat_interval_secs: u64,
        max_queue_size: usize,
    ) -> Self {
        Self {
            server_addr: server_addr.to_string(),
            auth_key: auth_key.to_string(),
            gateway_client: Arc::new(RwLock::new(None)),
            session_client: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(ClientState::Disconnected)),
            reconnect_interval: Duration::from_secs(reconnect_interval_secs),
            heartbeat_interval: Duration::from_secs(heartbeat_interval_secs),
            max_attempts: 0, // Infinite retries
            command_queue: Arc::new(RwLock::new(VecDeque::new())),
            max_queue_size,
            dropped_count: Arc::new(RwLock::new(0)),
            processed_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Get the current connection state
    pub async fn state(&self) -> ClientState {
        *self.state.read().await
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        matches!(self.state().await, ClientState::Connected)
    }

    /// Get the gateway management client (returns None if not connected)
    pub async fn gateway_client(&self) -> Option<GatewayManagementClient> {
        self.gateway_client.read().await.clone()
    }

    /// Get the session-to-server client (returns None if not connected)
    pub async fn session_client(&self) -> Option<SessionToWorldClient> {
        self.session_client.read().await.clone()
    }

    /// Get queue statistics
    pub async fn queue_stats(&self) -> QueueStats {
        let queue = self.command_queue.read().await;
        let dropped = *self.dropped_count.read().await;
        let processed = *self.processed_count.read().await;

        QueueStats {
            queued_count: queue.len(),
            processed_count: processed,
            dropped_count: dropped,
            max_queue_size: self.max_queue_size,
        }
    }

    /// Queue a command for later execution
    pub async fn queue_command(&self, session_id: String, command: String) -> Result<(), String> {
        let mut queue = self.command_queue.write().await;

        // Check if queue is full
        if self.max_queue_size > 0 && queue.len() >= self.max_queue_size {
            // Drop oldest command to make room
            queue.pop_front();
            let mut dropped = self.dropped_count.write().await;
            *dropped += 1;

            // Record metrics
            counter!("gateway_rpc_queue_dropped_total").increment(1);
            event!(
                Level::WARN,
                metric = "rpc_queue_dropped",
                "RPC command dropped from queue"
            );

            tracing::warn!(
                "Command queue full ({}), dropped oldest command for session {}",
                self.max_queue_size,
                session_id
            );
        }

        let queued_cmd = QueuedCommand {
            session_id: session_id.clone(),
            command: command.clone(),
            queued_at: std::time::Instant::now(),
        };

        queue.push_back(queued_cmd);

        // Record metrics
        gauge!("gateway_rpc_queue_size").set(queue.len() as f64);
        event!(Level::DEBUG, size = %queue.len(), metric = "rpc_queue_size", "RPC queue size");

        tracing::debug!(
            "Queued command for session {} (queue size: {}): {}",
            session_id,
            queue.len(),
            command
        );

        Ok(())
    }

    /// Process all queued commands
    async fn process_queued_commands(&self) {
        let mut queue = self.command_queue.write().await;
        let queue_size = queue.len();

        if queue_size == 0 {
            return;
        }

        tracing::info!("Processing {} queued commands", queue_size);

        // Reset processed count
        {
            let mut processed = self.processed_count.write().await;
            *processed = 0;
        }

        // Process all queued commands
        while let Some(queued_cmd) = queue.pop_front() {
            let age = queued_cmd.queued_at.elapsed();
            tracing::debug!(
                "Processing queued command for session {} (age: {:?}): {}",
                queued_cmd.session_id,
                age,
                queued_cmd.command
            );

            if let Some(mut client) = self.session_client().await {
                let request = SendInputRequest {
                    session_id: queued_cmd.session_id.clone(),
                    command: queued_cmd.command.clone(),
                };

                match client.send_input(request).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        if resp.success {
                            let mut processed = self.processed_count.write().await;
                            *processed += 1;
                            tracing::debug!(
                                "Successfully processed queued command for session {}",
                                queued_cmd.session_id
                            );
                        } else {
                            let error_msg =
                                resp.error.unwrap_or_else(|| "Unknown error".to_string());
                            tracing::warn!(
                                "Queued command failed for session {}: {}",
                                queued_cmd.session_id,
                                error_msg
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "RPC error processing queued command for session {}: {}",
                            queued_cmd.session_id,
                            e
                        );
                        // Re-queue the command and stop processing
                        queue.push_front(queued_cmd);
                        break;
                    }
                }
            } else {
                // No client available, re-queue and stop
                tracing::warn!("No client available, re-queuing commands");
                queue.push_front(queued_cmd);
                break;
            }
        }

        let final_processed = *self.processed_count.read().await;
        tracing::info!(
            "Finished processing queued commands: {} processed, {} remaining",
            final_processed,
            queue.len()
        );
    }

    /// Attempt to connect to the server
    async fn connect(&self) -> Result<(), String> {
        tracing::info!("Attempting to connect to server at {}", self.server_addr);

        // Update state
        {
            let mut state = self.state.write().await;
            *state = ClientState::Connecting;
        }

        // Record state change
        gauge!("gateway_rpc_connection_state").set(1.0);
        event!(
            Level::INFO,
            state = "connecting",
            metric = "rpc_connection_state",
            "RPC connection state"
        );

        // Attempt gRPC connection with timeout and retry configuration
        // Create a fresh endpoint for each connection attempt to avoid caching issues
        let endpoint = format!("http://{}", self.server_addr);
        match Channel::from_shared(endpoint.clone()) {
            Ok(endpoint) => {
                // Configure endpoint with timeout and connection settings
                // Note: We create a new channel each time to avoid connection pooling issues
                let endpoint = endpoint
                    .timeout(Duration::from_secs(5))
                    .connect_timeout(Duration::from_secs(5))
                    .tcp_keepalive(Some(Duration::from_secs(30)))
                    .http2_keep_alive_interval(Duration::from_secs(30))
                    .keep_alive_timeout(Duration::from_secs(10))
                    .initial_connection_window_size(Some(1024 * 1024))
                    .initial_stream_window_size(Some(1024 * 1024));

                match endpoint.connect().await {
                    Ok(channel) => {
                        tracing::info!("gRPC connection established to {}", self.server_addr);

                        // Create both clients from the same channel
                        let mut gateway_client = GatewayManagementClient::new(channel.clone());
                        let session_client = SessionToWorldClient::new(channel);

                        // Authenticate the gateway connection
                        tracing::info!("Authenticating gateway connection");
                        let auth_request = AuthenticateGatewayRequest {
                            auth_key: self.auth_key.clone(),
                        };

                        match gateway_client.authenticate_gateway(auth_request).await {
                            Ok(response) => {
                                let resp = response.into_inner();
                                if resp.success {
                                    tracing::info!("Gateway authentication successful");
                                } else {
                                    let error_msg =
                                        resp.error.unwrap_or_else(|| "Unknown error".to_string());
                                    tracing::error!("Gateway authentication failed: {}", error_msg);
                                    let mut state = self.state.write().await;
                                    *state = ClientState::Failed;
                                    return Err(format!("Authentication failed: {}", error_msg));
                                }
                            }
                            Err(e) => {
                                // Provide more specific error messages for connection issues
                                let error_msg = if e.code() == tonic::Code::Unavailable {
                                    format!(
                                        "Unable to connect to server at {}: service unavailable",
                                        self.server_addr
                                    )
                                } else if e.code() == tonic::Code::Unimplemented {
                                    format!(
                                        "Unable to connect to server at {}: authentication endpoint not available (server may be starting up)",
                                        self.server_addr
                                    )
                                } else {
                                    format!(
                                        "Unable to connect to server at {}: {}",
                                        self.server_addr, e
                                    )
                                };

                                // For Unavailable and Unimplemented errors, treat as temporary (Disconnected)
                                // These typically mean the server is starting up or not ready yet
                                let is_temporary = e.code() == tonic::Code::Unavailable
                                    || e.code() == tonic::Code::Unimplemented;

                                if is_temporary {
                                    tracing::warn!("{}", error_msg);
                                    let mut state = self.state.write().await;
                                    *state = ClientState::Disconnected;
                                } else {
                                    tracing::error!("{}", error_msg);
                                    let mut state = self.state.write().await;
                                    *state = ClientState::Failed;
                                }
                                return Err(error_msg);
                            }
                        }

                        // Store both clients
                        {
                            let mut gw_client = self.gateway_client.write().await;
                            *gw_client = Some(gateway_client);
                        }
                        {
                            let mut sess_client = self.session_client.write().await;
                            *sess_client = Some(session_client);
                        }

                        // Update state
                        {
                            let mut state = self.state.write().await;
                            *state = ClientState::Connected;
                        }

                        // Record state change
                        gauge!("gateway_rpc_connection_state").set(2.0);
                        event!(
                            Level::INFO,
                            state = "connected",
                            metric = "rpc_connection_state",
                            "RPC connection state"
                        );

                        tracing::info!("gRPC clients connected and authenticated successfully");
                        Ok(())
                    }
                    Err(e) => {
                        let error_msg =
                            format!("Unable to connect to server at {}: {}", self.server_addr, e);
                        tracing::error!("{}", error_msg);
                        let mut state = self.state.write().await;
                        *state = ClientState::Failed;
                        Err(error_msg)
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Invalid server address {}: {}", self.server_addr, e);
                tracing::error!("{}", error_msg);
                let mut state = self.state.write().await;
                *state = ClientState::Failed;
                Err(error_msg)
            }
        }
    }

    /// Start the reconnection loop
    pub async fn start_reconnection_loop(self: Arc<Self>) {
        let mut attempt = 0;

        loop {
            // Check current state
            let current_state = self.state().await;

            match current_state {
                ClientState::Disconnected | ClientState::Failed => {
                    attempt += 1;

                    // Check if we've exceeded max attempts
                    if self.max_attempts > 0 && attempt > self.max_attempts {
                        tracing::error!(
                            "Max reconnection attempts ({}) exceeded, giving up",
                            self.max_attempts
                        );
                        break;
                    }

                    tracing::info!(
                        "Reconnection attempt {} (interval: {:?})",
                        attempt,
                        self.reconnect_interval
                    );

                    // Attempt to connect
                    match self.connect().await {
                        Ok(()) => {
                            // Reset attempt counter on successful connection
                            attempt = 0;

                            tracing::info!("Successfully reconnected to server");

                            // Process any queued commands
                            self.process_queued_commands().await;
                        }
                        Err(e) => {
                            tracing::warn!("Reconnection attempt {} failed: {}", attempt, e);

                            // Clear any existing clients to force a fresh connection next time
                            {
                                let mut gw_client = self.gateway_client.write().await;
                                *gw_client = None;
                            }
                            {
                                let mut sess_client = self.session_client.write().await;
                                *sess_client = None;
                            }

                            // Reset state to Disconnected to allow next retry
                            {
                                let mut state = self.state.write().await;
                                *state = ClientState::Disconnected;
                            }

                            // Wait before next retry after failed connection
                            sleep(self.reconnect_interval).await;
                        }
                    }
                }
                ClientState::Connecting => {
                    // Wait a bit if we're in the middle of connecting
                    sleep(Duration::from_secs(1)).await;
                }
                ClientState::Connected => {
                    // Connection is healthy, wait before the next check
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Manually logout.txt
    pub async fn disconnect(&self) {
        tracing::info!("Manually disconnecting from server");

        // Clear both clients
        {
            let mut gw_client = self.gateway_client.write().await;
            *gw_client = None;
        }
        {
            let mut sess_client = self.session_client.write().await;
            *sess_client = None;
        }

        // Update state
        {
            let mut state = self.state.write().await;
            *state = ClientState::Disconnected;
        }
    }
    /// Send input, queuing it if disconnected
    pub async fn send_or_queue_input(
        &self,
        session_id: String,
        command: String,
    ) -> Result<(), String> {
        let start = Instant::now();

        if self.is_connected().await {
            // Try to send immediately
            if let Some(mut client) = self.session_client().await {
                let request = SendInputRequest {
                    session_id: session_id.clone(),
                    command: command.clone(),
                };

                match client.send_input(request).await {
                    Ok(response) => {
                        let duration = start.elapsed();
                        let resp = response.into_inner();
                        if resp.success {
                            // Record metrics
                            counter!("gateway_rpc_calls_total", "method" => "send_input", "success" => "true").increment(1);
                            histogram!("gateway_rpc_call_duration_seconds", "method" => "send_input").record(duration.as_secs_f64());
                            event!(Level::DEBUG, method = "send_input", duration_ms = %duration.as_millis(), metric = "rpc_call_success", "RPC call successful");

                            tracing::debug!("Command sent successfully for session {}", session_id);
                            return Ok(());
                        } else {
                            let error_msg =
                                resp.error.unwrap_or_else(|| "Unknown error".to_string());

                            // Record metrics
                            counter!("gateway_rpc_calls_total", "method" => "send_input", "success" => "false").increment(1);
                            counter!("gateway_rpc_errors_total", "method" => "send_input")
                                .increment(1);
                            histogram!("gateway_rpc_call_duration_seconds", "method" => "send_input").record(duration.as_secs_f64());
                            event!(Level::WARN, method = "send_input", duration_ms = %duration.as_millis(), metric = "rpc_call_error", "RPC call failed");

                            tracing::warn!(
                                "Command failed for session {}: {}",
                                session_id,
                                error_msg
                            );
                            return Err(format!("Command failed: {}", error_msg));
                        }
                    }
                    Err(e) => {
                        let duration = start.elapsed();

                        // Record metrics
                        counter!("gateway_rpc_calls_total", "method" => "send_input", "success" => "false").increment(1);
                        counter!("gateway_rpc_errors_total", "method" => "send_input").increment(1);
                        histogram!("gateway_rpc_call_duration_seconds", "method" => "send_input")
                            .record(duration.as_secs_f64());
                        event!(Level::WARN, method = "send_input", duration_ms = %duration.as_millis(), metric = "rpc_call_error", "RPC call failed");

                        tracing::warn!(
                            "RPC error sending command for session {}: {}, queuing command",
                            session_id,
                            e
                        );
                        // Mark as disconnected and queue
                        {
                            let mut state = self.state.write().await;
                            *state = ClientState::Disconnected;
                        }
                        return self.queue_command(session_id, command).await;
                    }
                }
            }
        }

        // Not connected, queue the command
        tracing::debug!("Not connected, queuing command for session {}", session_id);
        self.queue_command(session_id, command).await
    }

    /// Execute a command with automatic retry on failure (for session operations)
    pub async fn execute_session_with_retry<F, T, E>(&self, mut f: F) -> Result<T, String>
    where
        F: FnMut(
            &SessionToWorldClient,
        ) -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>>,
        E: std::fmt::Display,
    {
        const MAX_RETRIES: usize = 3;

        for attempt in 1..=MAX_RETRIES {
            if let Some(client) = self.session_client().await {
                match f(&client).await {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        tracing::warn!("RPC call failed (attempt {}): {}", attempt, e);

                        if attempt < MAX_RETRIES {
                            // Mark as disconnected to trigger reconnection
                            {
                                let mut state = self.state.write().await;
                                *state = ClientState::Disconnected;
                            }

                            // Wait a bit before retry
                            sleep(Duration::from_secs(1)).await;
                        } else {
                            return Err(format!(
                                "RPC call failed after {} attempts: {}",
                                MAX_RETRIES, e
                            ));
                        }
                    }
                }
            } else {
                tracing::warn!("No RPC client available (attempt {})", attempt);

                if attempt < MAX_RETRIES {
                    sleep(Duration::from_secs(1)).await;
                } else {
                    return Err("No RPC client available after retries".to_string());
                }
            }
        }

        Err("Failed to execute RPC call".to_string())
    }

    /// Start the heartbeat loop
    ///
    /// This sends periodic heartbeat messages to the server to keep the connection alive
    /// and detect connection failures. This is a gateway-level heartbeat that works
    /// independently of any user sessions.
    pub async fn start_heartbeat_loop(self: Arc<Self>, gateway_id: String) {
        tracing::info!(
            "Starting gateway heartbeat loop with interval: {:?}",
            self.heartbeat_interval
        );

        loop {
            // Wait for the heartbeat interval
            sleep(self.heartbeat_interval).await;

            // Only send heartbeat if connected
            if self.is_connected().await {
                if let Some(mut client) = self.gateway_client().await {
                    tracing::debug!("Sending gateway heartbeat from {}", gateway_id);

                    let request = GatewayHeartbeatRequest {
                        gateway_id: gateway_id.clone(),
                    };

                    match client.gateway_heartbeat(request).await {
                        Ok(response) => {
                            let resp = response.into_inner();
                            if resp.success {
                                tracing::debug!("Gateway heartbeat successful from {}", gateway_id);
                            } else {
                                let error_msg =
                                    resp.error.unwrap_or_else(|| "Unknown error".to_string());
                                tracing::warn!(
                                    "Gateway heartbeat failed from {}: {}",
                                    gateway_id,
                                    error_msg
                                );
                                // Mark as disconnected to trigger reconnection
                                let mut state = self.state.write().await;
                                *state = ClientState::Disconnected;
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Gateway heartbeat RPC error from {}: {}",
                                gateway_id,
                                e
                            );
                            // Mark as disconnected to trigger reconnection
                            let mut state = self.state.write().await;
                            *state = ClientState::Disconnected;
                        }
                    }
                } else {
                    tracing::debug!("Skipping gateway heartbeat - no client available");
                }
            } else {
                tracing::debug!(
                    "Skipping gateway heartbeat - not connected (state: {:?})",
                    self.state().await
                );
            }
        }
    }

    /// Check if a username is available
    pub async fn check_username(&self, username: String) -> Result<bool, String> {
        if let Some(mut client) = self.gateway_client().await {
            let request = CheckUsernameRequest { username };

            match client.check_username(request).await {
                Ok(response) => {
                    let resp = response.into_inner();
                    if let Some(error) = resp.error {
                        Err(error)
                    } else {
                        Ok(resp.available)
                    }
                }
                Err(e) => Err(format!("RPC error: {}", e)),
            }
        } else {
            Err(format!(
                "Unable to connect to server at {}: not connected",
                self.server_addr
            ))
        }
    }

    /// Create a new account
    pub async fn create_account(
        &self,
        address: String,
        username: String,
        password: String,
        email: Option<String>,
        display_name: Option<String>,
        discord: Option<String>,
        timezone: Option<String>,
    ) -> Result<wyldlands_common::proto::AccountInfo, String> {
        if let Some(mut client) = self.gateway_client().await {
            let properties = HashMap::from([
                ("email".to_string(), email.unwrap_or_default()),
                ("display".to_string(), display_name.unwrap_or_default()),
                ("discord".to_string(), discord.unwrap_or_default()),
                ("timezone".to_string(), timezone.unwrap_or_default()),
            ]);
            let request = CreateAccountRequest {
                address,
                username,
                password,
                properties,
            };

            match client.create_account(request).await {
                Ok(response) => {
                    let resp = response.into_inner();
                    if resp.success {
                        resp.account
                            .ok_or_else(|| "No account info returned".to_string())
                    } else {
                        Err(resp.error.unwrap_or_else(|| "Unknown error".to_string()))
                    }
                }
                Err(e) => Err(format!("RPC error: {}", e)),
            }
        } else {
            Err(format!(
                "Unable to connect to server at {}: not connected",
                self.server_addr
            ))
        }
    }

    /// Authenticate a session
    pub async fn authenticate_session(
        &self,
        session_id: String,
        username: String,
        password: String,
        client_addr: String,
    ) -> Result<wyldlands_common::proto::AccountInfo, String> {
        if let Some(mut client) = self.session_client().await {
            let request = AuthenticateSessionRequest {
                session_id,
                username,
                password,
                client_addr,
            };

            match client.authenticate_session(request).await {
                Ok(response) => {
                    let resp = response.into_inner();
                    if resp.success {
                        resp.account
                            .ok_or_else(|| "No account info returned".to_string())
                    } else {
                        Err(resp
                            .error
                            .unwrap_or_else(|| "Authentication failed".to_string()))
                    }
                }
                Err(e) => Err(format!("RPC error: {}", e)),
            }
        } else {
            Err(format!(
                "Unable to connect to server at {}: not connected",
                self.server_addr
            ))
        }
    }

    /// Authenticate a session
    pub async fn fetch_server_statistics(
        &self,
    ) -> Result<wyldlands_common::proto::ServerStatisticsResponse, String> {
        if let Some(mut client) = self.gateway_client().await {
            let request = ServerStatisticsRequest {
                statistics: vec![],
            };

            match client.fetch_server_statistics(request).await {
                Ok(response) => {
                    Ok(response.into_inner())
                }
                Err(e) => Err(format!("RPC error: {}", e)),
            }
        } else {
            Err(format!(
                "Unable to connect to server at {}: not connected",
                self.server_addr
            ))
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_manager_creation() {
        let addr = "127.0.0.1:6006";
        let auth_key = "test-key";
        let manager = RpcClientManager::new(addr, auth_key, 5, 30);

        assert_eq!(manager.state().await, ClientState::Disconnected);
        assert!(!manager.is_connected().await);
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let addr = "127.0.0.1:6006";
        let auth_key = "test-key";
        let manager = RpcClientManager::new(addr, auth_key, 5, 30);

        // Initial state
        assert_eq!(manager.state().await, ClientState::Disconnected);

        // Manual state change for testing
        {
            let mut state = manager.state.write().await;
            *state = ClientState::Connecting;
        }
        assert_eq!(manager.state().await, ClientState::Connecting);

        {
            let mut state = manager.state.write().await;
            *state = ClientState::Connected;
        }
        assert!(manager.is_connected().await);
    }

    #[tokio::test]
    async fn test_command_queuing() {
        let addr = "127.0.0.1:6006";
        let auth_key = "test-key";
        let manager = RpcClientManager::with_queue_size(addr, auth_key, 5, 30, 10);

        // Initially disconnected
        assert_eq!(manager.state().await, ClientState::Disconnected);

        // Queue some commands
        manager
            .queue_command("session1".to_string(), "look".to_string())
            .await
            .unwrap();
        manager
            .queue_command("session1".to_string(), "north".to_string())
            .await
            .unwrap();

        // Check queue stats
        let stats = manager.queue_stats().await;
        assert_eq!(stats.queued_count, 2);
        assert_eq!(stats.processed_count, 0);
        assert_eq!(stats.dropped_count, 0);
    }

    #[tokio::test]
    async fn test_queue_overflow() {
        let addr = "127.0.0.1:6006";
        let auth_key = "test-key";
        let manager = RpcClientManager::with_queue_size(addr, auth_key, 5, 30, 3);

        // Queue more commands than the limit
        for i in 0..5 {
            manager
                .queue_command("session1".to_string(), format!("command{}", i))
                .await
                .unwrap();
        }

        // Check that oldest commands were dropped
        let stats = manager.queue_stats().await;
        assert_eq!(stats.queued_count, 3);
        assert_eq!(stats.dropped_count, 2);
    }

    #[tokio::test]
    async fn test_send_command_or_queue_when_disconnected() {
        let addr = "127.0.0.1:6006";
        let auth_key = "test-key";
        let manager = RpcClientManager::new(addr, auth_key, 5, 30);

        // Should queue when disconnected
        manager
            .send_or_queue_input("session1".to_string(), "look".to_string())
            .await
            .unwrap();

        let stats = manager.queue_stats().await;
        assert_eq!(stats.queued_count, 1);
    }
}
