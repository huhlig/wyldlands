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

//! Telnet connection management

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

/// Telnet connection wrapper
pub struct TelnetConnection {
    /// Session ID
    session_id: Uuid,
    
    /// TCP stream
    stream: TcpStream,
    
    /// Client capabilities
    capabilities: ClientCapabilities,
}

/// Client capabilities negotiated during connection
#[derive(Debug, Clone, Default)]
pub struct ClientCapabilities {
    /// Supports MCCP compression
    pub mccp: bool,
    
    /// Supports MSDP protocol
    pub msdp: bool,
    
    /// Supports GMCP protocol
    pub gmcp: bool,
    
    /// Terminal window size (width, height)
    pub window_size: Option<(u16, u16)>,
    
    /// Terminal type
    pub terminal_type: Option<String>,
    
    /// Supports ANSI colors
    pub ansi_colors: bool,
}

impl TelnetConnection {
    /// Create a new telnet connection
    pub fn new(session_id: Uuid, stream: TcpStream) -> Self {
        Self {
            session_id,
            stream,
            capabilities: ClientCapabilities::default(),
        }
    }
    
    /// Get session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
    
    /// Get client capabilities
    pub fn capabilities(&self) -> &ClientCapabilities {
        &self.capabilities
    }
    
    /// Update client capabilities
    pub fn set_capabilities(&mut self, capabilities: ClientCapabilities) {
        self.capabilities = capabilities;
    }
    
    /// Send data to client
    pub async fn send(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        self.stream.write_all(data).await
    }
    
    /// Send text with CRLF line ending
    pub async fn send_line(&mut self, text: &str) -> Result<(), std::io::Error> {
        self.stream.write_all(text.as_bytes()).await?;
        self.stream.write_all(b"\r\n").await
    }
    
    /// Read data from client
    pub async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        self.stream.read(buffer).await
    }
    
    /// Flush the stream
    pub async fn flush(&mut self) -> Result<(), std::io::Error> {
        self.stream.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_client_capabilities_default() {
        let caps = ClientCapabilities::default();
        assert!(!caps.mccp);
        assert!(!caps.msdp);
        assert!(!caps.gmcp);
        assert!(caps.window_size.is_none());
        assert!(caps.terminal_type.is_none());
        assert!(!caps.ansi_colors);
    }
    
    #[test]
    fn test_client_capabilities_custom() {
        let caps = ClientCapabilities {
            mccp: true,
            msdp: true,
            gmcp: false,
            window_size: Some((80, 24)),
            terminal_type: Some("xterm-256color".to_string()),
            ansi_colors: true,
        };
        
        assert!(caps.mccp);
        assert!(caps.msdp);
        assert!(!caps.gmcp);
        assert_eq!(caps.window_size, Some((80, 24)));
        assert_eq!(caps.terminal_type.as_deref(), Some("xterm-256color"));
        assert!(caps.ansi_colors);
    }
}

