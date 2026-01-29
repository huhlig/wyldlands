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

//! Protocol adapter layer for translating between different client protocols
//! 
//! This module provides a unified interface for handling different client
//! protocols (Telnet, WebSocket) and translating them to a common format
//! for the game server.

use async_trait::async_trait;
use std::fmt;

pub mod telnet_adapter;
pub mod websocket_adapter;

/// Protocol message types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolMessage {
    /// Text message from client
    Text(String),
    
    /// Binary data from client
    Binary(Vec<u8>),
    
    /// Client connected
    Connected,
    
    /// Client disconnected
    Disconnected,
    
    /// Ping from client
    Ping,
    
    /// Pong from client
    Pong,
    
    /// Protocol negotiation data
    Negotiation(NegotiationData),
}

/// Protocol negotiation data
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NegotiationData {
    /// Terminal type
    TerminalType(String),
    
    /// Window size (width, height)
    WindowSize(u16, u16),
    
    /// Compression enabled
    CompressionEnabled,
    
    /// MSDP data
    MSDP(Vec<u8>),
    
    /// GMCP data
    GMCP(String),
}

/// Protocol adapter trait
/// 
/// Implementations of this trait handle protocol-specific details and
/// translate messages to/from a common format.
#[async_trait]
pub trait ProtocolAdapter: Send {
    /// Get the protocol name
    fn protocol_name(&self) -> &str;
    
    /// Send text to the client
    async fn send_text(&mut self, text: &str) -> Result<(), ProtocolError>;
    
    /// Send binary data to the client
    async fn send_binary(&mut self, data: &[u8]) -> Result<(), ProtocolError>;
    
    /// Send a line of text (with appropriate line ending)
    async fn send_line(&mut self, text: &str) -> Result<(), ProtocolError>;
    
    /// Receive a message from the client
    async fn receive(&mut self) -> Result<Option<ProtocolMessage>, ProtocolError>;
    
    /// Close the connection
    async fn close(&mut self) -> Result<(), ProtocolError>;
    
    /// Check if the connection is still alive
    fn is_alive(&self) -> bool;
    
    /// Get client capabilities
    fn capabilities(&self) -> ClientCapabilities;
}

/// Client capabilities across all protocols
#[derive(Debug, Clone, Default)]
pub struct ClientCapabilities {
    /// Supports compression
    pub compression: bool,
    
    /// Supports binary data
    pub binary: bool,
    
    /// Supports ANSI colors
    pub ansi_colors: bool,
    
    /// Terminal window size
    pub window_size: Option<(u16, u16)>,
    
    /// Terminal type
    pub terminal_type: Option<String>,
    
    /// Supports MSDP protocol
    pub msdp: bool,
    
    /// Supports GMCP protocol
    pub gmcp: bool,
}

/// Protocol adapter errors
#[derive(Debug)]
pub enum ProtocolError {
    /// IO error
    Io(std::io::Error),
    
    /// Connection closed
    ConnectionClosed,
    
    /// Protocol error
    ProtocolError(String),
    
    /// Unsupported operation
    Unsupported(String),
    
    /// Timeout
    Timeout,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::ConnectionClosed => write!(f, "Connection closed"),
            Self::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            Self::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            Self::Timeout => write!(f, "Timeout"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<std::io::Error> for ProtocolError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Helper function to strip ANSI codes from text
pub fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_escape = false;
    
    for ch in text.chars() {
        if ch == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Helper function to convert text to protocol-appropriate format
pub fn format_text_for_protocol(text: &str, supports_ansi: bool) -> String {
    if supports_ansi {
        text.to_string()
    } else {
        strip_ansi(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_strip_ansi() {
        let text = "\x1b[31mRed text\x1b[0m normal";
        let stripped = strip_ansi(text);
        assert_eq!(stripped, "Red text normal");
        
        let text = "No ANSI codes here";
        let stripped = strip_ansi(text);
        assert_eq!(stripped, "No ANSI codes here");
        
        let text = "\x1b[1m\x1b[32mBold green\x1b[0m";
        let stripped = strip_ansi(text);
        assert_eq!(stripped, "Bold green");
    }
    
    #[test]
    fn test_format_text_for_protocol() {
        let text = "\x1b[31mRed\x1b[0m";
        
        let formatted = format_text_for_protocol(text, true);
        assert_eq!(formatted, text);
        
        let formatted = format_text_for_protocol(text, false);
        assert_eq!(formatted, "Red");
    }
    
    #[test]
    fn test_client_capabilities_default() {
        let caps = ClientCapabilities::default();
        assert!(!caps.compression);
        assert!(!caps.binary);
        assert!(!caps.ansi_colors);
        assert!(caps.window_size.is_none());
        assert!(caps.terminal_type.is_none());
        assert!(!caps.msdp);
        assert!(!caps.gmcp);
    }
}

