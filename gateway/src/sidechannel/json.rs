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

//! WebSocket JSON side channel implementation
//!
//! This module provides JSON encoding for structured data over WebSocket connections.
//! Unlike MSDP and GMCP which are telnet-specific protocols, WebSocket JSON is a
//! simple JSON message format for web clients.
//!
//! ## Message Format
//!
//! Messages are sent as JSON objects with a `type` field and optional `data` field:
//! ```json
//! {
//!   "type": "room.info",
//!   "data": {
//!     "name": "Town Square",
//!     "description": "A bustling town square...",
//!     "exits": ["north", "south", "east", "west"]
//!   }
//! }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use wyldlands_common::proto::{DataArray, DataTable, DataValue, StructuredOutput};

/// WebSocket JSON error types
#[derive(Debug)]
pub enum WebSocketJsonError {
    JsonError(serde_json::Error),
    EncodingError(String),
    InvalidMessage(String),
}

impl std::fmt::Display for WebSocketJsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JsonError(e) => write!(f, "JSON error: {}", e),
            Self::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
            Self::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
        }
    }
}

impl std::error::Error for WebSocketJsonError {}

impl From<serde_json::Error> for WebSocketJsonError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err)
    }
}

/// WebSocket JSON message structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebSocketMessage {
    /// Message type (e.g., "room.info", "char.vitals", "combat.action")
    #[serde(rename = "type")]
    pub message_type: String,

    /// Optional message data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
}

impl WebSocketMessage {
    /// Create a new WebSocket message
    pub fn new(message_type: impl Into<String>, data: Option<JsonValue>) -> Self {
        Self {
            message_type: message_type.into(),
            data,
        }
    }

    /// Create a WebSocket message with data
    pub fn with_data(message_type: impl Into<String>, data: JsonValue) -> Self {
        Self {
            message_type: message_type.into(),
            data: Some(data),
        }
    }

    /// Create a WebSocket message without data
    pub fn without_data(message_type: impl Into<String>) -> Self {
        Self {
            message_type: message_type.into(),
            data: None,
        }
    }

    /// Encode the message to JSON string
    pub fn encode(&self) -> Result<String, WebSocketJsonError> {
        serde_json::to_string(self).map_err(Into::into)
    }

    /// Encode the message to pretty JSON string
    pub fn encode_pretty(&self) -> Result<String, WebSocketJsonError> {
        serde_json::to_string_pretty(self).map_err(Into::into)
    }

    /// Parse a WebSocket message from JSON string
    pub fn parse(json: &str) -> Result<Self, WebSocketJsonError> {
        serde_json::from_str(json).map_err(Into::into)
    }
}

/// Convert StructuredOutput proto to WebSocket JSON message
pub fn encode_structured_output(output: &StructuredOutput) -> Result<String, WebSocketJsonError> {
    // Convert the structured output to JSON
    let json_data = structured_output_to_json(output)?;

    // Create WebSocket message with the type from output_type
    let message_type = if output.output_type.is_empty() {
        "server.data".to_string()
    } else {
        output.output_type.clone()
    };

    let message = WebSocketMessage::with_data(message_type, json_data);
    message.encode()
}

/// Convert StructuredOutput to JSON value
fn structured_output_to_json(output: &StructuredOutput) -> Result<JsonValue, WebSocketJsonError> {
    if let Some(data) = &output.data {
        data_value_to_json(data)
    } else {
        Ok(JsonValue::Null)
    }
}

/// Convert DataValue proto to JSON value
fn data_value_to_json(value: &DataValue) -> Result<JsonValue, WebSocketJsonError> {
    use wyldlands_common::proto::data_value::DataValue as ProtoDataValue;

    match &value.data_value {
        Some(ProtoDataValue::StringData(s)) => Ok(json!(s)),
        Some(ProtoDataValue::TableData(table)) => data_table_to_json(table),
        Some(ProtoDataValue::ArrayData(array)) => data_array_to_json(array),
        None => Ok(JsonValue::Null),
    }
}

/// Convert DataTable proto to JSON object
fn data_table_to_json(table: &DataTable) -> Result<JsonValue, WebSocketJsonError> {
    let mut map = serde_json::Map::new();

    for (key, value) in &table.entries {
        let json_value = data_value_to_json(value)?;
        map.insert(key.clone(), json_value);
    }

    Ok(JsonValue::Object(map))
}

/// Convert DataArray proto to JSON array
fn data_array_to_json(array: &DataArray) -> Result<JsonValue, WebSocketJsonError> {
    let mut vec = Vec::new();

    for value in &array.values {
        let json_value = data_value_to_json(value)?;
        vec.push(json_value);
    }

    Ok(JsonValue::Array(vec))
}

/// Create a character vitals update message
pub fn create_vitals_update(
    health: i32,
    mana: i32,
    stamina: i32,
) -> Result<String, WebSocketJsonError> {
    let data = json!({
        "health": health,
        "mana": mana,
        "stamina": stamina
    });

    let message = WebSocketMessage::with_data("char.vitals", data);
    message.encode()
}

/// Create a room info message
pub fn create_room_info(
    name: &str,
    description: &str,
    exits: &[&str],
) -> Result<String, WebSocketJsonError> {
    let data = json!({
        "name": name,
        "description": description,
        "exits": exits
    });

    let message = WebSocketMessage::with_data("room.info", data);
    message.encode()
}

/// Create a combat action message
pub fn create_combat_action(
    actor: &str,
    action: &str,
    target: &str,
    damage: Option<i32>,
) -> Result<String, WebSocketJsonError> {
    let mut data = json!({
        "actor": actor,
        "action": action,
        "target": target
    });

    if let Some(dmg) = damage {
        data["damage"] = json!(dmg);
    }

    let message = WebSocketMessage::with_data("combat.action", data);
    message.encode()
}

/// Create an inventory update message
pub fn create_inventory_update(items: &[(&str, i32)]) -> Result<String, WebSocketJsonError> {
    let items_json: Vec<JsonValue> = items
        .iter()
        .map(|(name, quantity)| {
            json!({
                "name": name,
                "quantity": quantity
            })
        })
        .collect();

    let data = json!({ "items": items_json });
    let message = WebSocketMessage::with_data("inventory.update", data);
    message.encode()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_message_encode() {
        let message = WebSocketMessage::with_data("test.message", json!({"key": "value"}));
        let encoded = message.encode().unwrap();
        assert!(encoded.contains("\"type\":\"test.message\""));
        assert!(encoded.contains("\"key\":\"value\""));
    }

    #[test]
    fn test_websocket_message_parse() {
        let json = r#"{"type":"test.message","data":{"key":"value"}}"#;
        let message = WebSocketMessage::parse(json).unwrap();
        assert_eq!(message.message_type, "test.message");
        assert!(message.data.is_some());
        assert_eq!(message.data.unwrap()["key"], "value");
    }

    #[test]
    fn test_websocket_message_without_data() {
        let message = WebSocketMessage::without_data("ping");
        let encoded = message.encode().unwrap();
        assert!(encoded.contains("\"type\":\"ping\""));
        assert!(!encoded.contains("\"data\""));
    }

    #[test]
    fn test_create_vitals_update() {
        let encoded = create_vitals_update(100, 50, 75).unwrap();
        assert!(encoded.contains("\"type\":\"char.vitals\""));
        assert!(encoded.contains("\"health\":100"));
        assert!(encoded.contains("\"mana\":50"));
        assert!(encoded.contains("\"stamina\":75"));
    }

    #[test]
    fn test_create_room_info() {
        let encoded =
            create_room_info("Town Square", "A bustling square", &["north", "south"]).unwrap();
        assert!(encoded.contains("\"type\":\"room.info\""));
        assert!(encoded.contains("Town Square"));
        assert!(encoded.contains("north"));
    }

    #[test]
    fn test_create_combat_action() {
        let encoded = create_combat_action("Player", "attacks", "Goblin", Some(15)).unwrap();
        assert!(encoded.contains("\"type\":\"combat.action\""));
        assert!(encoded.contains("Player"));
        assert!(encoded.contains("attacks"));
        assert!(encoded.contains("Goblin"));
        assert!(encoded.contains("\"damage\":15"));
    }

    #[test]
    fn test_structured_output_to_websocket() {
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
            output_type: "char.vitals".to_string(),
            data: Some(DataValue {
                data_value: Some(ProtoDataValue::TableData(table)),
            }),
        };

        let encoded = encode_structured_output(&output).unwrap();
        assert!(encoded.contains("\"type\":\"char.vitals\""));
        assert!(encoded.contains("health"));
        assert!(encoded.contains("100"));
        assert!(encoded.contains("name"));
        assert!(encoded.contains("Player"));
    }

    #[test]
    fn test_roundtrip() {
        let original =
            WebSocketMessage::with_data("test.type", json!({"key": "value", "number": 42}));
        let encoded = original.encode().unwrap();
        let parsed = WebSocketMessage::parse(&encoded).unwrap();
        assert_eq!(original, parsed);
    }
}


