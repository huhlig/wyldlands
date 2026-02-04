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

//! WebSocket server adapter implementation

use crate::sidechannel::json::{self, WebSocketMessage};
use crate::server::{self, ClientCapabilities, ProtocolAdapter, ProtocolError, ProtocolMessage};
use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use futures::stream::StreamExt;
use wyldlands_common::proto::StructuredOutput;

/// Input mode for WebSocket adapter
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    /// Line-buffered input (Playing state)
    LineBuf,
    /// Keystroke-buffered input (Editing state)
    KeystrokeBuf {
        /// Title of what's being edited
        title: String,
        /// Current edit buffer content
        content: String,
    },
}

/// WebSocket server adapter
pub struct WebSocketAdapter {
    socket: WebSocket,
    capabilities: ClientCapabilities,
    alive: bool,
    input_mode: InputMode,
    line_buffer: String,
}

impl WebSocketAdapter {
    /// Create a new WebSocket adapter
    pub fn new(socket: WebSocket) -> Self {
        // WebSocket capabilities
        let capabilities = ClientCapabilities {
            compression: false, // Will be negotiated
            binary: true,       // WebSocket supports binary
            ansi_colors: true,  // Assume web clients support ANSI
            window_size: None,  // Not available via WebSocket
            terminal_type: Some("web".to_string()),
            msdp: false, // Not applicable for WebSocket
            gmcp: true,  // Can be implemented via JSON messages
        };

        Self {
            socket,
            capabilities,
            alive: true,
            input_mode: InputMode::LineBuf,
            line_buffer: String::new(),
        }
    }

    /// Enable compression
    pub fn enable_compression(&mut self) {
        self.capabilities.compression = true;
    }

    /// Set window size (if provided by client)
    pub fn set_window_size(&mut self, width: u16, height: u16) {
        self.capabilities.window_size = Some((width, height));
    }

    /// Switch to line-buffered input mode (Playing state)
    pub fn set_line_mode(&mut self) {
        self.input_mode = InputMode::LineBuf;
        self.line_buffer.clear();
    }

    /// Switch to keystroke-buffered input mode (Editing state)
    pub async fn set_editing_mode(
        &mut self,
        title: String,
        initial_content: String,
    ) -> Result<(), ProtocolError> {
        self.input_mode = InputMode::KeystrokeBuf {
            title: title.clone(),
            content: initial_content.clone(),
        };

        // Send editing instructions to client
        let instructions = format!(
            "\r\n=== Editing: {} ===\r\n\
            Press Ctrl+Enter to save, Ctrl+Escape to cancel\r\n\
            \r\n{}\r\n",
            title, initial_content
        );

        self.send_text(&instructions).await?;
        Ok(())
    }

    /// Get current edit content (for Editing mode)
    pub fn get_edit_content(&self) -> Option<String> {
        match &self.input_mode {
            InputMode::KeystrokeBuf { content, .. } => Some(content.clone()),
            _ => None,
        }
    }

    /// Clear edit buffer
    pub fn clear_edit_buffer(&mut self) {
        if let InputMode::KeystrokeBuf { title, .. } = &self.input_mode {
            self.input_mode = InputMode::KeystrokeBuf {
                title: title.clone(),
                content: String::new(),
            };
        }
    }

    /// Process input based on current mode
    fn process_input(&mut self, text: String) -> Option<String> {
        match &mut self.input_mode {
            InputMode::LineBuf => {
                // Line-buffered mode: accumulate until newline
                self.line_buffer.push_str(&text);

                if self.line_buffer.contains('\n') {
                    let line = self.line_buffer.trim().to_string();
                    self.line_buffer.clear();
                    Some(line)
                } else {
                    None
                }
            }
            InputMode::KeystrokeBuf { content, .. } => {
                // Keystroke-buffered mode: check for special commands
                // Ctrl+Enter (save) or Ctrl+Escape (cancel)
                if text.contains("\x13") || text == "@SAVE@" {
                    // Ctrl+S or save command
                    Some("@SAVE@".to_string())
                } else if text.contains("\x1b") || text == "@CANCEL@" {
                    // Escape or cancel command
                    Some("@CANCEL@".to_string())
                } else {
                    // Accumulate content
                    content.push_str(&text);
                    None
                }
            }
        }
    }

    /// Send structured output via WebSocket JSON
    ///
    /// Encodes the StructuredOutput to WebSocket JSON format and sends it
    pub async fn send_json_structured(
        &mut self,
        output: &StructuredOutput,
    ) -> Result<(), ProtocolError> {
        let encoded = json::encode_structured_output(output).map_err(|e| {
            ProtocolError::ProtocolError(format!("WebSocket JSON encoding error: {}", e))
        })?;

        self.send_text(&encoded).await
    }

    /// Send a WebSocket JSON message
    pub async fn send_json_message(
        &mut self,
        message: &WebSocketMessage,
    ) -> Result<(), ProtocolError> {
        let encoded = message.encode().map_err(|e| {
            ProtocolError::ProtocolError(format!("WebSocket JSON encoding error: {}", e))
        })?;

        self.send_text(&encoded).await
    }

    /// Send character vitals update
    pub async fn send_vitals_update(
        &mut self,
        health: i32,
        mana: i32,
        stamina: i32,
    ) -> Result<(), ProtocolError> {
        let encoded = json::create_vitals_update(health, mana, stamina).map_err(|e| {
            ProtocolError::ProtocolError(format!("WebSocket JSON encoding error: {}", e))
        })?;

        self.send_text(&encoded).await
    }

    /// Send room info update
    pub async fn send_room_info(
        &mut self,
        name: &str,
        description: &str,
        exits: &[&str],
    ) -> Result<(), ProtocolError> {
        let encoded = json::create_room_info(name, description, exits).map_err(|e| {
            ProtocolError::ProtocolError(format!("WebSocket JSON encoding error: {}", e))
        })?;

        self.send_text(&encoded).await
    }

    /// Send combat action
    pub async fn send_combat_action(
        &mut self,
        actor: &str,
        action: &str,
        target: &str,
        damage: Option<i32>,
    ) -> Result<(), ProtocolError> {
        let encoded =
            json::create_combat_action(actor, action, target, damage).map_err(|e| {
                ProtocolError::ProtocolError(format!("WebSocket JSON encoding error: {}", e))
            })?;

        self.send_text(&encoded).await
    }
}

#[async_trait]
impl ProtocolAdapter for WebSocketAdapter {
    fn protocol_name(&self) -> &str {
        "websocket"
    }

    async fn send_text(&mut self, text: &str) -> Result<(), ProtocolError> {
        self.socket
            .send(Message::Text(text.to_string().into()))
            .await
            .map_err(|e| {
                self.alive = false;
                ProtocolError::ProtocolError(format!("WebSocket send error: {}", e))
            })
    }

    async fn send_binary(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        self.socket
            .send(Message::Binary(data.to_vec().into()))
            .await
            .map_err(|e| {
                self.alive = false;
                ProtocolError::ProtocolError(format!("WebSocket send error: {}", e))
            })
    }

    async fn send_line(&mut self, text: &str) -> Result<(), ProtocolError> {
        // WebSocket doesn't need CRLF, just send with newline
        let line = format!("{}\r\n", text);
        self.send_text(&line).await
    }

    async fn receive(&mut self) -> Result<Option<ProtocolMessage>, ProtocolError> {
        match self.socket.next().await {
            Some(Ok(msg)) => {
                match msg {
                    Message::Text(text) => {
                        // Process input based on current mode
                        if let Some(processed) = self.process_input(text.to_string()) {
                            Ok(Some(ProtocolMessage::Text(processed)))
                        } else {
                            // Input buffered, wait for more
                            Ok(None)
                        }
                    }
                    Message::Binary(data) => Ok(Some(ProtocolMessage::Binary(data.to_vec()))),
                    Message::Ping(_) => Ok(Some(ProtocolMessage::Ping)),
                    Message::Pong(_) => Ok(Some(ProtocolMessage::Pong)),
                    Message::Close(_) => {
                        self.alive = false;
                        Ok(Some(ProtocolMessage::Disconnected))
                    }
                }
            }
            Some(Err(e)) => {
                self.alive = false;
                Err(ProtocolError::ProtocolError(format!(
                    "WebSocket error: {}",
                    e
                )))
            }
            None => {
                self.alive = false;
                Ok(Some(ProtocolMessage::Disconnected))
            }
        }
    }

    fn set_input_mode(&mut self, _mode: server::InputMode) {
        // WebSocket input mode is currently handled internally by process_input
    }

    async fn close(&mut self) -> Result<(), ProtocolError> {
        self.alive = false;
        self.socket
            .send(Message::Close(None))
            .await
            .map_err(|e| ProtocolError::ProtocolError(format!("WebSocket close error: {}", e)))
    }

    fn is_alive(&self) -> bool {
        self.alive
    }

    fn capabilities(&self) -> ClientCapabilities {
        self.capabilities.clone()
    }

    async fn send_structured(&mut self, output: &StructuredOutput) -> Result<(), ProtocolError> {
        // WebSocket always uses JSON for structured data
        self.send_json_structured(output).await
    }
}

#[cfg(test)]
mod tests {

    // Note: Full testing requires mock WebSocket
    // These are basic structure tests

    #[test]
    fn test_capabilities_default() {
        // This test would need a mock socket
        // Placeholder for structure
    }

    #[test]
    fn test_enable_compression() {
        // This test would need a mock socket
        // Placeholder for structure
    }
}
