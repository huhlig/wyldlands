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

//! WebSocket protocol adapter implementation

use crate::protocol::{
    ClientCapabilities, ProtocolAdapter, ProtocolError, ProtocolMessage,
};
use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use futures::stream::StreamExt;

/// WebSocket protocol adapter
pub struct WebSocketAdapter {
    socket: WebSocket,
    capabilities: ClientCapabilities,
    alive: bool,
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
            msdp: false,        // Not applicable for WebSocket
            gmcp: true,         // Can be implemented via JSON messages
        };
        
        Self {
            socket,
            capabilities,
            alive: true,
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
                    Message::Text(text) => Ok(Some(ProtocolMessage::Text(text.to_string()))),
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
                Err(ProtocolError::ProtocolError(format!("WebSocket error: {}", e)))
            }
            None => {
                self.alive = false;
                Ok(Some(ProtocolMessage::Disconnected))
            }
        }
    }
    
    async fn close(&mut self) -> Result<(), ProtocolError> {
        self.alive = false;
        self.socket
            .send(Message::Close(None))
            .await
            .map_err(|e| {
                ProtocolError::ProtocolError(format!("WebSocket close error: {}", e))
            })
    }
    
    fn is_alive(&self) -> bool {
        self.alive
    }
    
    fn capabilities(&self) -> ClientCapabilities {
        self.capabilities.clone()
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

