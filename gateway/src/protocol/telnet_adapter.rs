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

//! Telnet protocol adapter implementation
//! 
//! TEMPORARILY DISABLED: Waiting for termionix library availability

/*
use crate::protocol::{
    ClientCapabilities, ProtocolAdapter, ProtocolError, ProtocolMessage,
};
use crate::telnet::connection::TelnetConnection;
use async_trait::async_trait;

/// Telnet protocol adapter
pub struct TelnetAdapter {
    connection: TelnetConnection,
    capabilities: ClientCapabilities,
    alive: bool,
}

impl TelnetAdapter {
    /// Create a new telnet adapter
    pub fn new(connection: TelnetConnection) -> Self {
        // Initialize capabilities from telnet connection
        let telnet_caps = connection.capabilities();
        let capabilities = ClientCapabilities {
            compression: telnet_caps.mccp,
            binary: true, // Telnet supports binary
            ansi_colors: telnet_caps.ansi_colors,
            window_size: telnet_caps.window_size,
            terminal_type: telnet_caps.terminal_type.clone(),
            msdp: telnet_caps.msdp,
            gmcp: telnet_caps.gmcp,
        };
        
        Self {
            connection,
            capabilities,
            alive: true,
        }
    }
    
    /// Update capabilities from telnet connection
    pub fn update_capabilities(&mut self) {
        let telnet_caps = self.connection.capabilities();
        self.capabilities = ClientCapabilities {
            compression: telnet_caps.mccp,
            binary: true,
            ansi_colors: telnet_caps.ansi_colors,
            window_size: telnet_caps.window_size,
            terminal_type: telnet_caps.terminal_type.clone(),
            msdp: telnet_caps.msdp,
            gmcp: telnet_caps.gmcp,
        };
    }
}

#[async_trait]
impl ProtocolAdapter for TelnetAdapter {
    fn protocol_name(&self) -> &str {
        "telnet"
    }
    
    async fn send_text(&mut self, text: &str) -> Result<(), ProtocolError> {
        self.connection
            .send(text.as_bytes())
            .await
            .map_err(|e| {
                self.alive = false;
                ProtocolError::Io(e)
            })
    }
    
    async fn send_binary(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        self.connection
            .send(data)
            .await
            .map_err(|e| {
                self.alive = false;
                ProtocolError::Io(e)
            })
    }
    
    async fn send_line(&mut self, text: &str) -> Result<(), ProtocolError> {
        self.connection
            .send_line(text)
            .await
            .map_err(|e| {
                self.alive = false;
                ProtocolError::Io(e)
            })
    }
    
    async fn receive(&mut self) -> Result<Option<ProtocolMessage>, ProtocolError> {
        let mut buffer = vec![0u8; 4096];
        
        match self.connection.read(&mut buffer).await {
            Ok(0) => {
                // Connection closed
                self.alive = false;
                Ok(Some(ProtocolMessage::Disconnected))
            }
            Ok(n) => {
                // Process received data
                let data = &buffer[..n];
                
                // TODO: Parse telnet protocol sequences
                // For now, just convert to text
                let text = String::from_utf8_lossy(data).to_string();
                
                if text.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(ProtocolMessage::Text(text)))
                }
            }
            Err(e) => {
                self.alive = false;
                Err(ProtocolError::Io(e))
            }
        }
    }
    
    async fn close(&mut self) -> Result<(), ProtocolError> {
        self.alive = false;
        // Telnet connection will be closed when dropped
        Ok(())
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
    use super::*;
    
    // Note: Full testing requires mock TelnetConnection
    // These are basic structure tests
    
    #[test]
    fn test_protocol_name() {
        // This test would need a mock connection
        // Placeholder for structure
    }
}
*/

