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

//! Telnet protocol handler for the Wyldlands Gateway
//!
//! This module provides telnet protocol support using the termionix library,
//! including support for:
//! - Basic telnet protocol negotiation
//! - MCCP (MUD Client Compression Protocol)
//! - MSDP (MUD Server Data Protocol)
//! - GMCP (Generic MUD Communication Protocol)
//! - NAWS (Negotiate About Window Size)
//! - ANSI color codes

use crate::context::ServerContext;
use crate::reconnection::ReconnectionManager;
use crate::session::ProtocolType;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub mod character_sheet;
pub mod connection;
pub mod login;
pub mod protocol;

/// Telnet server configuration
#[derive(Debug, Clone)]
pub struct TelnetConfig {
    /// Enable MCCP compression
    pub enable_mccp: bool,

    /// Enable MSDP protocol
    pub enable_msdp: bool,

    /// Enable GMCP protocol
    pub enable_gmcp: bool,

    /// Enable NAWS (window size negotiation)
    pub enable_naws: bool,

    /// Enable ANSI colors
    pub enable_ansi: bool,

    /// Connection timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for TelnetConfig {
    fn default() -> Self {
        Self {
            enable_mccp: true,
            enable_msdp: true,
            enable_gmcp: true,
            enable_naws: true,
            enable_ansi: true,
            timeout_seconds: 300,
        }
    }
}

/// Telnet server
pub struct TelnetServer {
    context: ServerContext,
    config: TelnetConfig,
    reconnection_manager: Arc<ReconnectionManager>,
}

impl TelnetServer {
    /// Create a new telnet server
    pub fn new(context: ServerContext, config: TelnetConfig) -> Self {
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
    pub async fn run(self, listener: TcpListener) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Telnet server accepting connections...");

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    tracing::info!("New telnet connection from {}", addr);

                    let context = self.context.clone();
                    let config = self.config.clone();
                    let reconnection_manager = self.reconnection_manager.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(stream, addr, context, config, reconnection_manager)
                                .await
                        {
                            tracing::error!(
                                "Error handling telnet connection from {}: {}",
                                addr,
                                e
                            );
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Error accepting telnet connection: {}", e);
                }
            }
        }
    }
}

/// Handle a single telnet connection
async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    context: ServerContext,
    _config: TelnetConfig,
    reconnection_manager: Arc<ReconnectionManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("Handling telnet connection from {}", addr);

    // Create session
    let session_id = context
        .session_manager()
        .create_session(ProtocolType::Telnet, addr.to_string())
        .await
        .map_err(|e| format!("Failed to create session: {}", e))?;

    tracing::info!(
        "Created session {} for telnet connection from {}",
        session_id,
        addr
    );

    // Register connection in pool
    let _sender = context
        .connection_pool()
        .register(session_id, ProtocolType::Telnet)
        .await
        .map_err(|e| format!("Failed to register connection: {}", e))?;

    // Transition to authenticating state
    context
        .session_manager()
        .transition_session(
            session_id,
            wyldlands_common::session::SessionState::Authenticating,
        )
        .await
        .map_err(|e| format!("Failed to transition session: {}", e))?;

    // Main session loop - allows returning to character selection after exit
    loop {
        // Run login flow
        let mut login_handler = login::LoginHandler::new(session_id, context.clone());

        match login_handler.process(&mut stream).await {
            Ok(Some((account, avatar))) => {
            tracing::info!(
                "User {} logged in with avatar entity {} (session {})",
                account.login,
                avatar.entity_id,
                session_id
            );

            // Update session with entity_id
            if let Some(mut session) = context.session_manager().get_session(session_id).await {
                session.entity_id = Some(avatar.entity_id);
                context.session_manager().update_session(session).await?;
            }

            // Call select_character RPC to load character in world server
            let rpc_client = context.rpc_client();
            if let Some(client) = rpc_client.client().await {
                match client
                    .select_character(
                        tarpc::context::current(),
                        session_id.to_string(),
                        avatar.entity_id.to_string(),
                    )
                    .await
                {
                    Ok(Ok(_character_info)) => {
                        tracing::info!("Character loaded for session {}", session_id);
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Failed to select character: {:?}", e);
                        stream
                            .write_all(format!("Failed to load character: {:?}\r\n", e).as_bytes())
                            .await?;
                        continue; // Return to login loop
                    }
                    Err(e) => {
                        tracing::error!("RPC error selecting character: {}", e);
                        stream
                            .write_all(b"Server communication error. Please try again.\r\n")
                            .await?;
                        continue; // Return to login loop
                    }
                }
            } else {
                tracing::error!("No RPC client available for character selection");
                stream
                    .write_all(b"Server unavailable. Please try again later.\r\n")
                    .await?;
                continue; // Return to login loop
            }

            // Send welcome message
            stream
                .write_all(b"\r\nWelcome to the world!\r\n\r\n")
                .await?;

            // Generate and send reconnection token
            match reconnection_manager.generate_token(session_id).await {
                Ok(token) => match token.encode() {
                    Ok(encoded) => {
                        let token_msg = format!("Your reconnection token: {}\r\n\r\n", encoded);
                        stream.write_all(token_msg.as_bytes()).await?;
                        tracing::info!("Generated reconnection token for session {}", session_id);
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

            // Command processing loop
            let mut buffer = vec![0u8; 1024];
            let mut line_buffer = String::new();
            let mut return_to_char_selection = false;

            'playing: loop {
                match stream.read(&mut buffer).await {
                    Ok(0) => {
                        // Connection closed
                        tracing::info!("Telnet connection from {} closed", addr);

                        // Generate reconnection token for graceful disconnect
                        if let Ok(token) = reconnection_manager.generate_token(session_id).await {
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

                        break 'playing;
                    }
                    Ok(n) => {
                        let data = &buffer[..n];

                        // Process input character by character
                        for &byte in data {
                            match byte {
                                // Carriage return or newline - process command
                                b'\r' | b'\n' => {
                                    if !line_buffer.is_empty() {
                                        let command = line_buffer.trim().to_string();
                                        line_buffer.clear();

                                        tracing::debug!(
                                            "Processing command from {}: {}",
                                            addr,
                                            command
                                        );

                                        // Send command to server via RPC
                                        let rpc_client = context.rpc_client();
                                        if let Some(client) = rpc_client.client().await {
                                            match client
                                                .send_command(
                                                    tarpc::context::current(),
                                                    session_id.to_string(),
                                                    command.clone(),
                                                )
                                                .await
                                            {
                                                Ok(Ok(result)) => {
                                                    // Send command output back to client
                                                    for output in result.output {
                                                        match output {
                                                            wyldlands_common::gateway::GameOutput::Text(text) => {
                                                                stream.write_all(text.as_bytes()).await?;
                                                                stream.write_all(b"\r\n").await?;
                                                            }
                                                            wyldlands_common::gateway::GameOutput::FormattedText(text) => {
                                                                stream.write_all(text.as_bytes()).await?;
                                                                stream.write_all(b"\r\n").await?;
                                                            }
                                                            wyldlands_common::gateway::GameOutput::System(text) => {
                                                                // Check if this is the return to character selection signal
                                                                if text.contains("Returning to character selection") {
                                                                    return_to_char_selection = true;
                                                                }
                                                                stream.write_all(text.as_bytes()).await?;
                                                                stream.write_all(b"\r\n").await?;
                                                            }
                                                            _ => {
                                                                // Handle other output types as needed
                                                            }
                                                        }
                                                    }

                                                    if let Some(error) = result.error {
                                                        stream
                                                            .write_all(
                                                                format!("Error: {}\r\n", error)
                                                                    .as_bytes(),
                                                            )
                                                            .await?;
                                                    }

                                                    // If exit was signaled, break out of playing loop
                                                    if return_to_char_selection {
                                                        tracing::info!("Returning to character selection for session {}", session_id);
                                                        // Transition session state back to character selection
                                                        let _ = context
                                                            .session_manager()
                                                            .transition_session(
                                                                session_id,
                                                                wyldlands_common::session::SessionState::CharacterSelection,
                                                            )
                                                            .await;
                                                        break 'playing;
                                                    }
                                                }
                                                Ok(Err(e)) => {
                                                    tracing::error!("Command error: {:?}", e);
                                                    stream
                                                        .write_all(
                                                            format!("Command error: {:?}\r\n", e)
                                                                .as_bytes(),
                                                        )
                                                        .await?;
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        "Failed to send command to server: {}",
                                                        e
                                                    );
                                                    stream
                                                        .write_all(
                                                            b"Server error processing command\r\n",
                                                        )
                                                        .await?;
                                                }
                                            }
                                        } else {
                                            stream.write_all(b"Server not connected\r\n").await?;
                                        }

                                        // Send prompt
                                        stream.write_all(b"> ").await?;
                                    }
                                }
                                // Backspace or delete
                                127 | 8 => {
                                    if !line_buffer.is_empty() {
                                        line_buffer.pop();
                                        // Send backspace sequence to client
                                        stream.write_all(b"\x08 \x08").await?;
                                    }
                                }
                                // Printable characters
                                32..=126 => {
                                    line_buffer.push(byte as char);
                                    // Echo character back to client
                                    stream.write_all(&[byte]).await?;
                                }
                                // Ignore other control characters
                                _ => {}
                            }
                        }

                        stream.flush().await?;
                    }
                    Err(e) => {
                        tracing::error!("Error reading from telnet connection {}: {}", addr, e);
                        break 'playing;
                    }
                }
            }

            // If we exited the playing loop to return to character selection, continue the outer loop
            if return_to_char_selection {
                continue; // Goes back to login flow (which will show character selection)
            }

            // Otherwise, connection was closed, so break out of the main loop
            break;
        }
        Ok(None) => {
            tracing::info!("Login cancelled for session {}", session_id);
            stream.write_all(b"\r\nGoodbye!\r\n").await?;
            break;
        }
        Err(e) => {
            tracing::warn!("Login failed for session {}: {}", session_id, e);
            stream
                .write_all(format!("\r\n{}\r\n", e).as_bytes())
                .await?;
            break;
        }
    }
    } // End of main session loop

    // Unregister connection
    context
        .connection_pool()
        .unregister(session_id)
        .await
        .map_err(|e| format!("Failed to unregister connection: {}", e))?;

    tracing::info!("Telnet connection from {} fully closed", addr);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telnet_config_default() {
        let config = TelnetConfig::default();
        assert!(config.enable_mccp);
        assert!(config.enable_msdp);
        assert!(config.enable_gmcp);
        assert!(config.enable_naws);
        assert!(config.enable_ansi);
        assert_eq!(config.timeout_seconds, 300);
    }

    #[test]
    fn test_telnet_config_custom() {
        let config = TelnetConfig {
            enable_mccp: false,
            enable_msdp: true,
            enable_gmcp: false,
            enable_naws: true,
            enable_ansi: true,
            timeout_seconds: 600,
        };

        assert!(!config.enable_mccp);
        assert!(config.enable_msdp);
        assert!(!config.enable_gmcp);
        assert_eq!(config.timeout_seconds, 600);
    }
}


