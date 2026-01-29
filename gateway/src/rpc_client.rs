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

use std::collections::VecDeque;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tonic::transport::Channel;
use wyldlands_common::proto::{
    AuthenticateGatewayRequest, GatewayHeartbeatRequest, SendCommandRequest,
    gateway_server_client::GatewayServerClient,
};

// Type alias for the client with Channel transport
type GrpcClient = GatewayServerClient<Channel>;

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

    /// Current client (if connected)
    client: Arc<RwLock<Option<GrpcClient>>>,

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
            client: Arc::new(RwLock::new(None)),
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

    /// Get a client handle (returns None if not connected)
    pub async fn client(&self) -> Option<GrpcClient> {
        self.client.read().await.clone()
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

            if let Some(mut client) = self.client().await {
                let request = SendCommandRequest {
                    session_id: queued_cmd.session_id.clone(),
                    command: queued_cmd.command.clone(),
                };

                match client.send_command(request).await {
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
    async fn connect(&self) -> Result<GrpcClient, String> {
        tracing::info!("Attempting to connect to server at {}", self.server_addr);

        // Update state
        {
            let mut state = self.state.write().await;
            *state = ClientState::Connecting;
        }

        // Attempt gRPC connection
        let endpoint = format!("http://{}", self.server_addr);
        match Channel::from_shared(endpoint.clone()) {
            Ok(endpoint) => {
                match endpoint.connect().await {
                    Ok(channel) => {
                        tracing::info!("gRPC connection established to {}", self.server_addr);

                        // Create client
                        let mut client = GatewayServerClient::new(channel);

                        // Authenticate the gateway connection
                        tracing::info!("Authenticating gateway connection");
                        let auth_request = AuthenticateGatewayRequest {
                            auth_key: self.auth_key.clone(),
                        };

                        match client.authenticate_gateway(auth_request).await {
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
                                tracing::error!("Gateway authentication RPC error: {}", e);
                                let mut state = self.state.write().await;
                                *state = ClientState::Failed;
                                return Err(format!("Authentication RPC error: {}", e));
                            }
                        }

                        // Update state
                        {
                            let mut state = self.state.write().await;
                            *state = ClientState::Connected;
                        }

                        tracing::info!("gRPC client connected and authenticated successfully");
                        Ok(client)
                    }
                    Err(e) => {
                        tracing::error!("Failed to connect to server: {}", e);
                        let mut state = self.state.write().await;
                        *state = ClientState::Failed;
                        Err(format!("Connection failed: {}", e))
                    }
                }
            }
            Err(e) => {
                tracing::error!("Invalid endpoint {}: {}", endpoint, e);
                let mut state = self.state.write().await;
                *state = ClientState::Failed;
                Err(format!("Invalid endpoint: {}", e))
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
                        Ok(client) => {
                            // Store the client
                            {
                                let mut client_lock = self.client.write().await;
                                *client_lock = Some(client);
                            }

                            // Reset attempt counter on successful connection
                            attempt = 0;

                            tracing::info!("Successfully reconnected to server");

                            // Process any queued commands
                            self.process_queued_commands().await;
                        }
                        Err(e) => {
                            tracing::warn!("Reconnection attempt {} failed: {}", attempt, e);
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

            // Wait before next check/attempt
            if !matches!(current_state, ClientState::Connected) {
                sleep(self.reconnect_interval).await;
            }
        }
    }

    /// Manually disconnect
    pub async fn disconnect(&self) {
        tracing::info!("Manually disconnecting from server");

        // Clear client
        {
            let mut client = self.client.write().await;
            *client = None;
        }

        // Update state
        {
            let mut state = self.state.write().await;
            *state = ClientState::Disconnected;
        }
    }
    /// Send a command, queuing it if disconnected
    pub async fn send_command_or_queue(
        &self,
        session_id: String,
        command: String,
    ) -> Result<(), String> {
        if self.is_connected().await {
            // Try to send immediately
            if let Some(mut client) = self.client().await {
                let request = SendCommandRequest {
                    session_id: session_id.clone(),
                    command: command.clone(),
                };

                match client.send_command(request).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        if resp.success {
                            tracing::debug!("Command sent successfully for session {}", session_id);
                            return Ok(());
                        } else {
                            let error_msg =
                                resp.error.unwrap_or_else(|| "Unknown error".to_string());
                            tracing::warn!(
                                "Command failed for session {}: {}",
                                session_id,
                                error_msg
                            );
                            return Err(format!("Command failed: {}", error_msg));
                        }
                    }
                    Err(e) => {
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

    /// Execute a command with automatic retry on failure
    pub async fn execute_with_retry<F, T, E>(&self, mut f: F) -> Result<T, String>
    where
        F: FnMut(&GrpcClient) -> std::pin::Pin<Box<dyn Future<Output = Result<T, E>> + Send>>,
        E: std::fmt::Display,
    {
        const MAX_RETRIES: usize = 3;

        for attempt in 1..=MAX_RETRIES {
            if let Some(client) = self.client().await {
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
                if let Some(mut client) = self.client().await {
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
            .send_command_or_queue("session1".to_string(), "look".to_string())
            .await
            .unwrap();

        let stats = manager.queue_stats().await;
        assert_eq!(stats.queued_count, 1);
    }
}
