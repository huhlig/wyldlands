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

//! MSDP (Mud Server Data Protocol) Implementation
//!
//! This module implements the MSDP protocol for sending structured data
//! to MUD clients over telnet. MSDP uses telnet option 69.
//!
//! Reference: https://tintin.mudhalla.net/protocols/msdp/

use std::collections::HashMap;
use wyldlands_common::proto::{DataArray, DataTable, DataValue, StructuredOutput};

/// MSDP telnet option number
pub const MSDP: u8 = 69;

/// MSDP protocol constants
pub const MSDP_VAR: u8 = 1;
pub const MSDP_VAL: u8 = 2;
pub const MSDP_TABLE_OPEN: u8 = 3;
pub const MSDP_TABLE_CLOSE: u8 = 4;
pub const MSDP_ARRAY_OPEN: u8 = 5;
pub const MSDP_ARRAY_CLOSE: u8 = 6;

/// Telnet protocol constants
pub const IAC: u8 = 255;
pub const SB: u8 = 250;
pub const SE: u8 = 240;

/// MSDP encoding errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsdpError {
    /// Invalid MSDP data format
    InvalidFormat(String),
    /// Unexpected end of data
    UnexpectedEnd,
    /// Invalid variable name
    InvalidVariable(String),
    /// Nested structure too deep
    TooDeep,
}

impl std::fmt::Display for MsdpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(msg) => write!(f, "Invalid MSDP format: {}", msg),
            Self::UnexpectedEnd => write!(f, "Unexpected end of MSDP data"),
            Self::InvalidVariable(var) => write!(f, "Invalid MSDP variable: {}", var),
            Self::TooDeep => write!(f, "MSDP structure nested too deep"),
        }
    }
}

impl std::error::Error for MsdpError {}

/// MSDP command types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsdpCommand {
    /// LIST <type> - Request a list of supported items
    List(String),
    /// REPORT <var1> <var2> ... - Request continuous reporting of variables
    Report(Vec<String>),
    /// SEND <var1> <var2> ... - Request one-time send of variables
    Send(Vec<String>),
    /// UNREPORT <var1> <var2> ... - Stop reporting variables
    Unreport(Vec<String>),
    /// RESET <type> - Reset a group of variables
    Reset(String),
}

/// Parsed MSDP data structure
#[derive(Debug, Clone, PartialEq)]
pub enum MsdpValue {
    /// String value
    String(String),
    /// Table (map) of values
    Table(HashMap<String, MsdpValue>),
    /// Array of values
    Array(Vec<MsdpValue>),
}

/// Encode a StructuredOutput to MSDP binary format
///
/// Returns the complete MSDP subnegotiation including IAC SB MSDP ... IAC SE
pub fn encode_structured_output(output: &StructuredOutput) -> Result<Vec<u8>, MsdpError> {
    let mut result = Vec::new();

    // Start telnet subnegotiation: IAC SB MSDP
    result.push(IAC);
    result.push(SB);
    result.push(MSDP);

    // Encode the variable name (output_type)
    result.push(MSDP_VAR);
    result.extend_from_slice(output.output_type.as_bytes());

    // Encode the value
    result.push(MSDP_VAL);
    if let Some(ref data) = output.data {
        encode_data_value(&mut result, data)?;
    }

    // End telnet subnegotiation: IAC SE
    result.push(IAC);
    result.push(SE);

    Ok(result)
}

/// Encode a DataValue to MSDP format
fn encode_data_value(buffer: &mut Vec<u8>, value: &DataValue) -> Result<(), MsdpError> {
    match value.data_value.as_ref() {
        Some(wyldlands_common::proto::data_value::DataValue::StringData(s)) => {
            buffer.extend_from_slice(s.as_bytes());
        }
        Some(wyldlands_common::proto::data_value::DataValue::TableData(table)) => {
            encode_data_table(buffer, table)?;
        }
        Some(wyldlands_common::proto::data_value::DataValue::ArrayData(array)) => {
            encode_data_array(buffer, array)?;
        }
        None => {
            // Empty value
        }
    }
    Ok(())
}

/// Encode a DataTable to MSDP format
fn encode_data_table(buffer: &mut Vec<u8>, table: &DataTable) -> Result<(), MsdpError> {
    buffer.push(MSDP_TABLE_OPEN);

    for (key, value) in &table.entries {
        buffer.push(MSDP_VAR);
        buffer.extend_from_slice(key.as_bytes());
        buffer.push(MSDP_VAL);
        encode_data_value(buffer, value)?;
    }

    buffer.push(MSDP_TABLE_CLOSE);
    Ok(())
}

/// Encode a DataArray to MSDP format
fn encode_data_array(buffer: &mut Vec<u8>, array: &DataArray) -> Result<(), MsdpError> {
    buffer.push(MSDP_ARRAY_OPEN);

    for value in &array.values {
        buffer.push(MSDP_VAL);
        encode_data_value(buffer, value)?;
    }

    buffer.push(MSDP_ARRAY_CLOSE);
    Ok(())
}

/// Parse MSDP command from client input
///
/// Expects data without IAC SB MSDP prefix and IAC SE suffix
pub fn parse_msdp_command(data: &[u8]) -> Result<MsdpCommand, MsdpError> {
    if data.is_empty() {
        return Err(MsdpError::InvalidFormat("Empty MSDP data".to_string()));
    }

    // Parse the command structure: MSDP_VAR <command> MSDP_VAL <args>
    let mut pos = 0;

    // Expect MSDP_VAR
    if data[pos] != MSDP_VAR {
        return Err(MsdpError::InvalidFormat("Expected MSDP_VAR".to_string()));
    }
    pos += 1;

    // Read command name
    let command_start = pos;
    while pos < data.len() && data[pos] != MSDP_VAL {
        pos += 1;
    }

    if pos >= data.len() {
        return Err(MsdpError::UnexpectedEnd);
    }

    let command_name = String::from_utf8_lossy(&data[command_start..pos]).to_string();
    pos += 1; // Skip MSDP_VAL

    // Parse arguments based on command type
    match command_name.to_uppercase().as_str() {
        "LIST" => {
            let arg = String::from_utf8_lossy(&data[pos..]).to_string();
            Ok(MsdpCommand::List(arg))
        }
        "REPORT" => {
            let vars = parse_variable_list(&data[pos..])?;
            Ok(MsdpCommand::Report(vars))
        }
        "SEND" => {
            let vars = parse_variable_list(&data[pos..])?;
            Ok(MsdpCommand::Send(vars))
        }
        "UNREPORT" => {
            let vars = parse_variable_list(&data[pos..])?;
            Ok(MsdpCommand::Unreport(vars))
        }
        "RESET" => {
            let arg = String::from_utf8_lossy(&data[pos..]).to_string();
            Ok(MsdpCommand::Reset(arg))
        }
        _ => Err(MsdpError::InvalidVariable(command_name)),
    }
}

/// Parse a list of variables from MSDP data
///
/// Variables are sent as: MSDP_VAL <var1> MSDP_VAL <var2> ...
fn parse_variable_list(data: &[u8]) -> Result<Vec<String>, MsdpError> {
    let mut vars = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Find next MSDP_VAL or end
        let start = pos;
        while pos < data.len() && data[pos] != MSDP_VAL {
            pos += 1;
        }

        if start < pos {
            let var = String::from_utf8_lossy(&data[start..pos]).to_string();
            if !var.is_empty() {
                vars.push(var);
            }
        }

        if pos < data.len() {
            pos += 1; // Skip MSDP_VAL
        }
    }

    Ok(vars)
}

/// Create an MSDP response for a LIST command
pub fn create_list_response(list_type: &str, items: &[&str]) -> Result<Vec<u8>, MsdpError> {
    let mut result = Vec::new();

    // IAC SB MSDP
    result.push(IAC);
    result.push(SB);
    result.push(MSDP);

    // Variable name (the list type)
    result.push(MSDP_VAR);
    result.extend_from_slice(list_type.as_bytes());

    // Array of items
    result.push(MSDP_VAL);
    result.push(MSDP_ARRAY_OPEN);

    for item in items {
        result.push(MSDP_VAL);
        result.extend_from_slice(item.as_bytes());
    }

    result.push(MSDP_ARRAY_CLOSE);

    // IAC SE
    result.push(IAC);
    result.push(SE);

    Ok(result)
}

/// Create an MSDP variable update
pub fn create_variable_update(var_name: &str, value: &str) -> Result<Vec<u8>, MsdpError> {
    let mut result = Vec::new();

    // IAC SB MSDP
    result.push(IAC);
    result.push(SB);
    result.push(MSDP);

    // Variable and value
    result.push(MSDP_VAR);
    result.extend_from_slice(var_name.as_bytes());
    result.push(MSDP_VAL);
    result.extend_from_slice(value.as_bytes());

    // IAC SE
    result.push(IAC);
    result.push(SE);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wyldlands_common::proto::data_value;

    #[test]
    fn test_encode_simple_string() {
        let output = StructuredOutput {
            output_type: "TEST".to_string(),
            data: Some(DataValue {
                data_value: Some(data_value::DataValue::StringData("hello".to_string())),
            }),
        };

        let encoded = encode_structured_output(&output).unwrap();

        // Should contain: IAC SB MSDP MSDP_VAR "TEST" MSDP_VAL "hello" IAC SE
        assert_eq!(encoded[0], IAC);
        assert_eq!(encoded[1], SB);
        assert_eq!(encoded[2], MSDP);
        assert_eq!(encoded[3], MSDP_VAR);
        assert_eq!(&encoded[4..8], b"TEST");
        assert_eq!(encoded[8], MSDP_VAL);
        assert_eq!(&encoded[9..14], b"hello");
        assert_eq!(encoded[14], IAC);
        assert_eq!(encoded[15], SE);
    }

    #[test]
    fn test_encode_table() {
        let mut entries = HashMap::new();
        entries.insert(
            "HEALTH".to_string(),
            DataValue {
                data_value: Some(data_value::DataValue::StringData("100".to_string())),
            },
        );
        entries.insert(
            "MANA".to_string(),
            DataValue {
                data_value: Some(data_value::DataValue::StringData("50".to_string())),
            },
        );

        let output = StructuredOutput {
            output_type: "CHARACTER".to_string(),
            data: Some(DataValue {
                data_value: Some(data_value::DataValue::TableData(DataTable { entries })),
            }),
        };

        let encoded = encode_structured_output(&output).unwrap();

        // Verify structure
        assert_eq!(encoded[0], IAC);
        assert_eq!(encoded[1], SB);
        assert_eq!(encoded[2], MSDP);

        // Should contain MSDP_TABLE_OPEN and MSDP_TABLE_CLOSE
        assert!(encoded.contains(&MSDP_TABLE_OPEN));
        assert!(encoded.contains(&MSDP_TABLE_CLOSE));

        // Should end with IAC SE
        assert_eq!(encoded[encoded.len() - 2], IAC);
        assert_eq!(encoded[encoded.len() - 1], SE);
    }

    #[test]
    fn test_encode_array() {
        let values = vec![
            DataValue {
                data_value: Some(data_value::DataValue::StringData("north".to_string())),
            },
            DataValue {
                data_value: Some(data_value::DataValue::StringData("south".to_string())),
            },
        ];

        let output = StructuredOutput {
            output_type: "EXITS".to_string(),
            data: Some(DataValue {
                data_value: Some(data_value::DataValue::ArrayData(DataArray { values })),
            }),
        };

        let encoded = encode_structured_output(&output).unwrap();

        // Should contain MSDP_ARRAY_OPEN and MSDP_ARRAY_CLOSE
        assert!(encoded.contains(&MSDP_ARRAY_OPEN));
        assert!(encoded.contains(&MSDP_ARRAY_CLOSE));
    }

    #[test]
    fn test_parse_list_command() {
        let data = b"\x01LIST\x02COMMANDS";
        let cmd = parse_msdp_command(data).unwrap();

        assert_eq!(cmd, MsdpCommand::List("COMMANDS".to_string()));
    }

    #[test]
    fn test_parse_report_command() {
        let data = b"\x01REPORT\x02HEALTH\x02MANA";
        let cmd = parse_msdp_command(data).unwrap();

        match cmd {
            MsdpCommand::Report(vars) => {
                assert_eq!(vars.len(), 2);
                assert!(vars.contains(&"HEALTH".to_string()));
                assert!(vars.contains(&"MANA".to_string()));
            }
            _ => panic!("Expected Report command"),
        }
    }

    #[test]
    fn test_create_list_response() {
        let items = vec!["LIST", "REPORT", "SEND", "UNREPORT", "RESET"];
        let response = create_list_response("COMMANDS", &items).unwrap();

        // Verify structure
        assert_eq!(response[0], IAC);
        assert_eq!(response[1], SB);
        assert_eq!(response[2], MSDP);
        assert!(response.contains(&MSDP_ARRAY_OPEN));
        assert!(response.contains(&MSDP_ARRAY_CLOSE));
        assert_eq!(response[response.len() - 2], IAC);
        assert_eq!(response[response.len() - 1], SE);
    }

    #[test]
    fn test_create_variable_update() {
        let update = create_variable_update("HEALTH", "85").unwrap();

        // Should be: IAC SB MSDP MSDP_VAR "HEALTH" MSDP_VAL "85" IAC SE
        assert_eq!(update[0], IAC);
        assert_eq!(update[1], SB);
        assert_eq!(update[2], MSDP);
        assert_eq!(update[3], MSDP_VAR);
        assert_eq!(&update[4..10], b"HEALTH");
        assert_eq!(update[10], MSDP_VAL);
        assert_eq!(&update[11..13], b"85");
        assert_eq!(update[13], IAC);
        assert_eq!(update[14], SE);
    }
}


