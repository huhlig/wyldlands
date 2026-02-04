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

//! GMCP (Generic Mud Communication Protocol) implementation
//!
//! GMCP is a telnet option (201) that uses JSON for structured data exchange.
//! It provides a more modern alternative to MSDP with better type support.
//!
//! ## Protocol Overview
//!
//! GMCP messages have the format:
//! ```text
//! IAC SB GMCP <Package.Subpackage.Command> <JSON data> IAC SE
//! ```
//!
//! The package name is case-insensitive (except for MSDP over GMCP which is case-sensitive).
//! The JSON data is optional and separated from the package by a space.
//!
//! ## MSDP over GMCP
//!
//! GMCP supports MSDP commands using the "MSDP" package name:
//! ```text
//! IAC SB GMCP MSDP {"LIST": "COMMANDS"} IAC SE
//! ```
//!
//! This allows MSDP-capable servers to communicate with GMCP-only clients.

use serde_json::{Value as JsonValue, json};
use wyldlands_common::proto::{DataArray, DataTable, DataValue, StructuredOutput};

/// GMCP telnet option code
pub const GMCP: u8 = 201;

/// GMCP error types
#[derive(Debug)]
pub enum GmcpError {
    InvalidFormat(String),
    JsonError(serde_json::Error),
    MissingPackage,
    InvalidPackage(String),
    EncodingError(String),
}

impl std::fmt::Display for GmcpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(msg) => write!(f, "Invalid GMCP format: {}", msg),
            Self::JsonError(e) => write!(f, "JSON parse error: {}", e),
            Self::MissingPackage => write!(f, "Missing package name"),
            Self::InvalidPackage(name) => write!(f, "Invalid package name: {}", name),
            Self::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
        }
    }
}

impl std::error::Error for GmcpError {}

impl From<serde_json::Error> for GmcpError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err)
    }
}

/// GMCP message structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GmcpMessage {
    /// Package name (e.g., "Core.Hello", "Char.Vitals", "MSDP")
    pub package: String,

    /// JSON data (optional)
    pub data: Option<JsonValue>,
}

impl GmcpMessage {
    /// Create a new GMCP message
    pub fn new(package: impl Into<String>, data: Option<JsonValue>) -> Self {
        Self {
            package: package.into(),
            data,
        }
    }

    /// Create a GMCP message with JSON data
    pub fn with_data(package: impl Into<String>, data: JsonValue) -> Self {
        Self {
            package: package.into(),
            data: Some(data),
        }
    }

    /// Create a GMCP message without data
    pub fn without_data(package: impl Into<String>) -> Self {
        Self {
            package: package.into(),
            data: None,
        }
    }

    /// Encode the message to GMCP format (without IAC SB/SE wrapper)
    pub fn encode(&self) -> Result<Vec<u8>, GmcpError> {
        let mut result = Vec::new();

        // Add package name
        result.extend_from_slice(self.package.as_bytes());

        // Add data if present
        if let Some(data) = &self.data {
            result.push(b' ');
            let json_str = serde_json::to_string(data)?;
            result.extend_from_slice(json_str.as_bytes());
        }

        Ok(result)
    }

    /// Parse a GMCP message from bytes (without IAC SB/SE wrapper)
    pub fn parse(data: &[u8]) -> Result<Self, GmcpError> {
        let text = String::from_utf8_lossy(data);
        let text = text.trim();

        if text.is_empty() {
            return Err(GmcpError::MissingPackage);
        }

        // Find the space separating package from data
        if let Some(space_pos) = text.find(' ') {
            let package = text[..space_pos].to_string();
            let json_str = &text[space_pos + 1..];

            // Parse JSON data
            let data = serde_json::from_str(json_str)?;

            Ok(Self {
                package,
                data: Some(data),
            })
        } else {
            // No data, just package name
            Ok(Self {
                package: text.to_string(),
                data: None,
            })
        }
    }
}

/// Convert StructuredOutput proto to GMCP JSON
pub fn encode_structured_output(output: &StructuredOutput) -> Result<Vec<u8>, GmcpError> {
    // Convert the structured output to JSON
    let json_data = structured_output_to_json(output)?;

    // Create GMCP message with the package name from the output_type
    let package = if output.output_type.is_empty() {
        "Server.Data".to_string()
    } else {
        output.output_type.clone()
    };

    let message = GmcpMessage::with_data(package, json_data);
    message.encode()
}

/// Convert StructuredOutput to JSON value
fn structured_output_to_json(output: &StructuredOutput) -> Result<JsonValue, GmcpError> {
    if let Some(data) = &output.data {
        data_value_to_json(data)
    } else {
        Ok(JsonValue::Null)
    }
}

/// Convert DataValue proto to JSON value
fn data_value_to_json(value: &DataValue) -> Result<JsonValue, GmcpError> {
    use wyldlands_common::proto::data_value::DataValue as ProtoDataValue;

    match &value.data_value {
        Some(ProtoDataValue::StringData(s)) => Ok(json!(s)),
        Some(ProtoDataValue::TableData(table)) => data_table_to_json(table),
        Some(ProtoDataValue::ArrayData(array)) => data_array_to_json(array),
        None => Ok(JsonValue::Null),
    }
}

/// Convert DataTable proto to JSON object
fn data_table_to_json(table: &DataTable) -> Result<JsonValue, GmcpError> {
    let mut map = serde_json::Map::new();

    for (key, value) in &table.entries {
        let json_value = data_value_to_json(value)?;
        map.insert(key.clone(), json_value);
    }

    Ok(JsonValue::Object(map))
}

/// Convert DataArray proto to JSON array
fn data_array_to_json(array: &DataArray) -> Result<JsonValue, GmcpError> {
    let mut vec = Vec::new();

    for value in &array.values {
        let json_value = data_value_to_json(value)?;
        vec.push(json_value);
    }

    Ok(JsonValue::Array(vec))
}

/// Create a GMCP Core.Hello message
pub fn create_hello_message(client_name: &str, client_version: &str) -> Result<Vec<u8>, GmcpError> {
    let data = json!({
        "client": client_name,
        "version": client_version
    });

    let message = GmcpMessage::with_data("Core.Hello", data);
    message.encode()
}

/// Create a GMCP Core.Supports.Set message
pub fn create_supports_set(packages: &[&str]) -> Result<Vec<u8>, GmcpError> {
    let data = json!(packages);
    let message = GmcpMessage::with_data("Core.Supports.Set", data);
    message.encode()
}

/// Create a GMCP Core.Supports.Add message
pub fn create_supports_add(packages: &[&str]) -> Result<Vec<u8>, GmcpError> {
    let data = json!(packages);
    let message = GmcpMessage::with_data("Core.Supports.Add", data);
    message.encode()
}

/// Create a GMCP Core.Supports.Remove message
pub fn create_supports_remove(packages: &[&str]) -> Result<Vec<u8>, GmcpError> {
    let data = json!(packages);
    let message = GmcpMessage::with_data("Core.Supports.Remove", data);
    message.encode()
}

/// Create a GMCP variable update message
pub fn create_variable_update(
    package: &str,
    key: &str,
    value: JsonValue,
) -> Result<Vec<u8>, GmcpError> {
    let data = json!({ key: value });
    let message = GmcpMessage::with_data(package, data);
    message.encode()
}

/// Create an MSDP over GMCP message
pub fn create_msdp_over_gmcp(msdp_data: JsonValue) -> Result<Vec<u8>, GmcpError> {
    let message = GmcpMessage::with_data("MSDP", msdp_data);
    message.encode()
}

/// Parse a GMCP message and extract package and data
pub fn parse_gmcp_message(data: &[u8]) -> Result<GmcpMessage, GmcpError> {
    GmcpMessage::parse(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gmcp_message_encode_with_data() {
        let data = json!({"client": "TestClient", "version": "1.0"});
        let message = GmcpMessage::with_data("Core.Hello", data);
        let encoded = message.encode().unwrap();
        let text = String::from_utf8(encoded).unwrap();
        assert!(text.starts_with("Core.Hello "));
        assert!(text.contains("TestClient"));
    }

    #[test]
    fn test_gmcp_message_encode_without_data() {
        let message = GmcpMessage::without_data("Core.Ping");
        let encoded = message.encode().unwrap();
        let text = String::from_utf8(encoded).unwrap();
        assert_eq!(text, "Core.Ping");
    }

    #[test]
    fn test_gmcp_message_parse_with_data() {
        let input = b"Core.Hello {\"client\":\"TestClient\",\"version\":\"1.0\"}";
        let message = GmcpMessage::parse(input).unwrap();
        assert_eq!(message.package, "Core.Hello");
        assert!(message.data.is_some());
        let data = message.data.unwrap();
        assert_eq!(data["client"], "TestClient");
        assert_eq!(data["version"], "1.0");
    }

    #[test]
    fn test_gmcp_message_parse_without_data() {
        let input = b"Core.Ping";
        let message = GmcpMessage::parse(input).unwrap();
        assert_eq!(message.package, "Core.Ping");
        assert!(message.data.is_none());
    }

    #[test]
    fn test_create_hello_message() {
        let encoded = create_hello_message("Wyldlands", "1.0").unwrap();
        let text = String::from_utf8(encoded).unwrap();
        assert!(text.starts_with("Core.Hello "));
        assert!(text.contains("Wyldlands"));
    }

    #[test]
    fn test_create_supports_set() {
        let packages = vec!["Core", "Char", "Room"];
        let encoded = create_supports_set(&packages).unwrap();
        let text = String::from_utf8(encoded).unwrap();
        assert!(text.starts_with("Core.Supports.Set "));
        assert!(text.contains("Core"));
        assert!(text.contains("Char"));
        assert!(text.contains("Room"));
    }

    #[test]
    fn test_structured_output_to_gmcp() {
        use wyldlands_common::proto::data_value::DataValue as ProtoDataValue;

        let mut table = DataTable::default();
        table.entries.insert(
            "health".to_string(),
            DataValue {
                data_value: Some(ProtoDataValue::StringData("100".to_string())),
            },
        );
        table.entries.insert(
            "name".to_string(),
            DataValue {
                data_value: Some(ProtoDataValue::StringData("Player".to_string())),
            },
        );

        let output = StructuredOutput {
            output_type: "Char.Vitals".to_string(),
            data: Some(DataValue {
                data_value: Some(ProtoDataValue::TableData(table)),
            }),
        };

        let encoded = encode_structured_output(&output).unwrap();
        let text = String::from_utf8(encoded).unwrap();
        assert!(text.starts_with("Char.Vitals ") || text.starts_with("Server.Data "));
        assert!(text.contains("health"));
        assert!(text.contains("100"));
        assert!(text.contains("name"));
        assert!(text.contains("Player"));
    }

    #[test]
    fn test_msdp_over_gmcp() {
        let msdp_data = json!({"LIST": "COMMANDS"});
        let encoded = create_msdp_over_gmcp(msdp_data).unwrap();
        let text = String::from_utf8(encoded).unwrap();
        assert!(text.starts_with("MSDP "));
        assert!(text.contains("LIST"));
        assert!(text.contains("COMMANDS"));
    }

    #[test]
    fn test_roundtrip() {
        let original =
            GmcpMessage::with_data("Test.Package", json!({"key": "value", "number": 42}));
        let encoded = original.encode().unwrap();
        let parsed = GmcpMessage::parse(&encoded).unwrap();
        assert_eq!(original, parsed);
    }
}


