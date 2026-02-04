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

//! Termionix-based telnet adapter implementation

use crate::server::{ClientCapabilities, InputMode, ProtocolAdapter, ProtocolError, ProtocolMessage};
use async_trait::async_trait;
use termionix_server::{ConnectionId, TelnetConnection, TerminalEvent};
use tokio::sync::mpsc;
use wyldlands_common::proto::StructuredOutput;

// Import MSDP and GMCP from crate root (re-exported from sidechannel module)
use crate::sidechannel::gmcp::{self, GmcpMessage};
use crate::sidechannel::msdp::{self, MsdpCommand};

/// Adapter that wraps a Termionix TelnetConnection to implement ProtocolAdapter
pub struct TermionixAdapter {
    connection: TelnetConnection,
    capabilities: ClientCapabilities,
    event_receiver: mpsc::UnboundedReceiver<TerminalEvent>,
    input_mode: InputMode,
    alive: bool,
}

impl TermionixAdapter {
    /// Create a new adapter from a Termionix connection
    pub fn new(
        connection: TelnetConnection,
        event_receiver: mpsc::UnboundedReceiver<TerminalEvent>,
    ) -> Self {
        // Extract capabilities from connection
        let capabilities = ClientCapabilities {
            compression: false,  // TODO: Get from connection
            binary: false,       // TODO: Get from connection
            ansi_colors: true,   // Termionix supports ANSI
            window_size: None,   // TODO: Get from connection
            terminal_type: None, // TODO: Get from connection
            msdp: false,         // TODO: Get from connection
            gmcp: false,         // TODO: Get from connection
        };

        Self {
            connection,
            capabilities,
            event_receiver,
            input_mode: InputMode::Line,
            alive: true,
        }
    }

    /// Get the connection ID
    pub fn connection_id(&self) -> ConnectionId {
        self.connection.id()
    }

    /// Update capabilities from terminal events
    fn update_capabilities_from_event(&mut self, event: &TerminalEvent) {
        match event {
            TerminalEvent::WindowSize { width, height } => {
                self.capabilities.window_size = Some((*width, *height));
            }
            TerminalEvent::TerminalType { terminal_type } => {
                self.capabilities.terminal_type = Some(terminal_type.clone());
            }
            _ => {}
        }
    }

    /// Send MSDP data to the client
    ///
    /// This sends a complete MSDP subnegotiation (IAC SB MSDP ... IAC SE)
    pub async fn send_msdp(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        // MSDP data is binary, send it directly
        self.send_binary(data).await
    }

    /// Send structured output via MSDP
    ///
    /// Encodes the StructuredOutput to MSDP format and sends it
    pub async fn send_msdp_structured(
        &mut self,
        output: &StructuredOutput,
    ) -> Result<(), ProtocolError> {
        if !self.capabilities.msdp {
            return Err(ProtocolError::Unsupported("MSDP not enabled".to_string()));
        }

        let encoded = msdp::encode_structured_output(output)
            .map_err(|e| ProtocolError::ProtocolError(format!("MSDP encoding error: {}", e)))?;

        self.send_msdp(&encoded).await
    }

    /// Send an MSDP variable update
    pub async fn send_msdp_variable(
        &mut self,
        var_name: &str,
        value: &str,
    ) -> Result<(), ProtocolError> {
        if !self.capabilities.msdp {
            return Err(ProtocolError::Unsupported("MSDP not enabled".to_string()));
        }

        let encoded = msdp::create_variable_update(var_name, value)
            .map_err(|e| ProtocolError::ProtocolError(format!("MSDP encoding error: {}", e)))?;

        self.send_msdp(&encoded).await
    }

    /// Send an MSDP list response
    pub async fn send_msdp_list(
        &mut self,
        list_type: &str,
        items: &[&str],
    ) -> Result<(), ProtocolError> {
        if !self.capabilities.msdp {
            return Err(ProtocolError::Unsupported("MSDP not enabled".to_string()));
        }

        let encoded = msdp::create_list_response(list_type, items)
            .map_err(|e| ProtocolError::ProtocolError(format!("MSDP encoding error: {}", e)))?;

        self.send_msdp(&encoded).await
    }

    /// Enable MSDP capability
    ///
    /// Called when MSDP negotiation succeeds
    pub fn enable_msdp(&mut self) {
        self.capabilities.msdp = true;
        tracing::info!("MSDP enabled for connection {}", self.connection.id());
    }

    /// Disable MSDP capability
    pub fn disable_msdp(&mut self) {
        self.capabilities.msdp = false;
        tracing::info!("MSDP disabled for connection {}", self.connection.id());
    }

    /// Process an MSDP command from the client
    ///
    /// Returns the parsed command for handling by the application
    pub fn process_msdp_command(&self, data: &[u8]) -> Result<MsdpCommand, ProtocolError> {
        msdp::parse_msdp_command(data)
            .map_err(|e| ProtocolError::ProtocolError(format!("MSDP parse error: {}", e)))
    }

    /// Send GMCP data to the client
    ///
    /// This sends a complete GMCP subnegotiation (IAC SB GMCP ... IAC SE)
    pub async fn send_gmcp(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        // GMCP data is text-based JSON, send it directly
        self.send_binary(data).await
    }

    /// Send structured output via GMCP
    ///
    /// Encodes the StructuredOutput to GMCP format and sends it
    pub async fn send_gmcp_structured(
        &mut self,
        output: &StructuredOutput,
    ) -> Result<(), ProtocolError> {
        if !self.capabilities.gmcp {
            return Err(ProtocolError::Unsupported("GMCP not enabled".to_string()));
        }

        let encoded = gmcp::encode_structured_output(output)
            .map_err(|e| ProtocolError::ProtocolError(format!("GMCP encoding error: {}", e)))?;

        self.send_gmcp(&encoded).await
    }

    /// Send a GMCP message
    pub async fn send_gmcp_message(&mut self, message: &GmcpMessage) -> Result<(), ProtocolError> {
        if !self.capabilities.gmcp {
            return Err(ProtocolError::Unsupported("GMCP not enabled".to_string()));
        }

        let encoded = message
            .encode()
            .map_err(|e| ProtocolError::ProtocolError(format!("GMCP encoding error: {}", e)))?;

        self.send_gmcp(&encoded).await
    }

    /// Send a GMCP Core.Hello message
    pub async fn send_gmcp_hello(
        &mut self,
        client_name: &str,
        version: &str,
    ) -> Result<(), ProtocolError> {
        if !self.capabilities.gmcp {
            return Err(ProtocolError::Unsupported("GMCP not enabled".to_string()));
        }

        let encoded = gmcp::create_hello_message(client_name, version)
            .map_err(|e| ProtocolError::ProtocolError(format!("GMCP encoding error: {}", e)))?;

        self.send_gmcp(&encoded).await
    }

    /// Send a GMCP Core.Supports.Set message
    pub async fn send_gmcp_supports(&mut self, packages: &[&str]) -> Result<(), ProtocolError> {
        if !self.capabilities.gmcp {
            return Err(ProtocolError::Unsupported("GMCP not enabled".to_string()));
        }

        let encoded = gmcp::create_supports_set(packages)
            .map_err(|e| ProtocolError::ProtocolError(format!("GMCP encoding error: {}", e)))?;

        self.send_gmcp(&encoded).await
    }

    /// Enable GMCP capability
    ///
    /// Called when GMCP negotiation succeeds
    pub fn enable_gmcp(&mut self) {
        self.capabilities.gmcp = true;
        tracing::info!("GMCP enabled for connection {}", self.connection.id());
    }

    /// Disable GMCP capability
    pub fn disable_gmcp(&mut self) {
        self.capabilities.gmcp = false;
        tracing::info!("GMCP disabled for connection {}", self.connection.id());
    }

    /// Process a GMCP message from the client
    ///
    /// Returns the parsed message for handling by the application
    pub fn process_gmcp_message(&self, data: &[u8]) -> Result<GmcpMessage, ProtocolError> {
        gmcp::parse_gmcp_message(data)
            .map_err(|e| ProtocolError::ProtocolError(format!("GMCP parse error: {}", e)))
    }
}

#[async_trait]
impl ProtocolAdapter for TermionixAdapter {
    fn protocol_name(&self) -> &str {
        "termionix-telnet"
    }

    async fn send_text(&mut self, text: &str) -> Result<(), ProtocolError> {
        self.connection.send(text, true).await.map_err(|e| {
            self.alive = false;
            ProtocolError::ProtocolError(e.to_string())
        })
    }

    async fn send_binary(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        // Convert binary data to string for sending
        // Termionix handles binary data through its terminal codec
        let text = String::from_utf8_lossy(data).to_string();
        self.connection.send(text.as_str(), true).await.map_err(|e| {
            self.alive = false;
            ProtocolError::ProtocolError(e.to_string())
        })
    }

    async fn send_line(&mut self, text: &str) -> Result<(), ProtocolError> {
        // Termionix handles line endings internally
        let line = if text.ends_with("\r\n") {
            text.to_string()
        } else {
            format!("{}\r\n", text.trim_end_matches('\n').trim_end_matches('\r'))
        };

        self.connection.send(&line, true).await.map_err(|e| {
            self.alive = false;
            ProtocolError::ProtocolError(e.to_string())
        })
    }

    async fn flush(&mut self) -> Result<(), ProtocolError> {
        self.connection.flush().await.map_err(|e| {
            self.alive = false;
            ProtocolError::ProtocolError(e.to_string())
        })
    }

    async fn receive(&mut self) -> Result<Option<ProtocolMessage>, ProtocolError> {
        loop {
            match self.event_receiver.recv().await {
                Some(event) => {
                    // Update capabilities if needed
                    self.update_capabilities_from_event(&event);

                    match event {
                        TerminalEvent::LineCompleted { line, .. } => {
                            // Convert SegmentedString to plain string
                            return Ok(Some(ProtocolMessage::Text(line.to_string())));
                        }
                        TerminalEvent::CharacterData { character, .. } => {
                            // Only return character data if in character mode
                            if self.input_mode == InputMode::Character {
                                return Ok(Some(ProtocolMessage::Text(character.to_string())));
                            }
                            // Otherwise, ignore individual characters and keep waiting
                            continue;
                        }
                        TerminalEvent::Disconnected => {
                            self.alive = false;
                            return Ok(Some(ProtocolMessage::Disconnected));
                        }
                        TerminalEvent::WindowSize { width, height } => {
                            return Ok(Some(ProtocolMessage::Negotiation(
                                crate::server::NegotiationData::WindowSize(width, height),
                            )));
                        }
                        TerminalEvent::TerminalType { terminal_type } => {
                            return Ok(Some(ProtocolMessage::Negotiation(
                                crate::server::NegotiationData::TerminalType(terminal_type),
                            )));
                        }
                        _ => {
                            // Ignore other events and keep waiting
                            continue;
                        }
                    }
                }
                None => {
                    // Channel closed
                    self.alive = false;
                    return Ok(Some(ProtocolMessage::Disconnected));
                }
            }
        }
    }

    fn set_input_mode(&mut self, mode: InputMode) {
        self.input_mode = mode;
    }

    async fn close(&mut self) -> Result<(), ProtocolError> {
        self.alive = false;
        // Termionix handles connection cleanup automatically
        Ok(())
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn capabilities(&self) -> ClientCapabilities {
        self.capabilities.clone()
    }

    async fn send_structured(&mut self, output: &StructuredOutput) -> Result<(), ProtocolError> {
        // Route to appropriate side channel based on capabilities
        if self.capabilities.gmcp {
            self.send_gmcp_structured(output).await
        } else if self.capabilities.msdp {
            self.send_msdp_structured(output).await
        } else {
            // Fallback to plain text
            let text = format!("[{}]\n", output.output_type);
            self.send_text(&text).await
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_adapter_creation() {
        // This would need a mock connection
        // Placeholder for structure
    }
}


