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

//! Telnet protocol constants and utilities
//! 
//! This module defines telnet protocol commands, options, and helper functions
//! for protocol negotiation and data handling.

/// Telnet command codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TelnetCommand {
    /// Interpret As Command
    IAC = 255,
    /// Don't do option
    DONT = 254,
    /// Do option
    DO = 253,
    /// Won't do option
    WONT = 252,
    /// Will do option
    WILL = 251,
    /// Subnegotiation begin
    SB = 250,
    /// Go ahead
    GA = 249,
    /// Erase line
    EL = 248,
    /// Erase character
    EC = 247,
    /// Are you there
    AYT = 246,
    /// Abort output
    AO = 245,
    /// Interrupt process
    IP = 244,
    /// Break
    BRK = 243,
    /// Data mark
    DM = 242,
    /// No operation
    NOP = 241,
    /// Subnegotiation end
    SE = 240,
}

impl TelnetCommand {
    /// Convert byte to telnet command
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            255 => Some(Self::IAC),
            254 => Some(Self::DONT),
            253 => Some(Self::DO),
            252 => Some(Self::WONT),
            251 => Some(Self::WILL),
            250 => Some(Self::SB),
            249 => Some(Self::GA),
            248 => Some(Self::EL),
            247 => Some(Self::EC),
            246 => Some(Self::AYT),
            245 => Some(Self::AO),
            244 => Some(Self::IP),
            243 => Some(Self::BRK),
            242 => Some(Self::DM),
            241 => Some(Self::NOP),
            240 => Some(Self::SE),
            _ => None,
        }
    }
    
    /// Convert command to byte
    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

/// Telnet option codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TelnetOption {
    /// Binary transmission
    Binary = 0,
    /// Echo
    Echo = 1,
    /// Suppress go ahead
    SuppressGoAhead = 3,
    /// Status
    Status = 5,
    /// Timing mark
    TimingMark = 6,
    /// Terminal type
    TerminalType = 24,
    /// Negotiate about window size (NAWS)
    NAWS = 31,
    /// Terminal speed
    TerminalSpeed = 32,
    /// Remote flow control
    RemoteFlowControl = 33,
    /// Linemode
    Linemode = 34,
    /// Environment variables
    EnvironmentVariables = 36,
    /// MCCP2 (MUD Client Compression Protocol v2)
    MCCP2 = 86,
    /// MCCP3 (MUD Client Compression Protocol v3)
    MCCP3 = 87,
    /// MSDP (MUD Server Data Protocol)
    MSDP = 69,
    /// GMCP (Generic MUD Communication Protocol)
    GMCP = 201,
}

impl TelnetOption {
    /// Convert byte to telnet option
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Binary),
            1 => Some(Self::Echo),
            3 => Some(Self::SuppressGoAhead),
            5 => Some(Self::Status),
            6 => Some(Self::TimingMark),
            24 => Some(Self::TerminalType),
            31 => Some(Self::NAWS),
            32 => Some(Self::TerminalSpeed),
            33 => Some(Self::RemoteFlowControl),
            34 => Some(Self::Linemode),
            36 => Some(Self::EnvironmentVariables),
            69 => Some(Self::MSDP),
            86 => Some(Self::MCCP2),
            87 => Some(Self::MCCP3),
            201 => Some(Self::GMCP),
            _ => None,
        }
    }
    
    /// Convert option to byte
    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

/// Build a telnet negotiation sequence
pub fn build_negotiation(command: TelnetCommand, option: TelnetOption) -> Vec<u8> {
    vec![
        TelnetCommand::IAC.to_byte(),
        command.to_byte(),
        option.to_byte(),
    ]
}

/// Build a telnet subnegotiation sequence
pub fn build_subnegotiation(option: TelnetOption, data: &[u8]) -> Vec<u8> {
    let mut result = vec![
        TelnetCommand::IAC.to_byte(),
        TelnetCommand::SB.to_byte(),
        option.to_byte(),
    ];
    
    // Escape IAC bytes in data
    for &byte in data {
        result.push(byte);
        if byte == TelnetCommand::IAC.to_byte() {
            result.push(byte); // Double IAC for escaping
        }
    }
    
    result.push(TelnetCommand::IAC.to_byte());
    result.push(TelnetCommand::SE.to_byte());
    
    result
}

/// Parse window size from NAWS subnegotiation data
pub fn parse_window_size(data: &[u8]) -> Option<(u16, u16)> {
    if data.len() >= 4 {
        let width = u16::from_be_bytes([data[0], data[1]]);
        let height = u16::from_be_bytes([data[2], data[3]]);
        Some((width, height))
    } else {
        None
    }
}

/// ANSI color codes
pub mod ansi {
    /// Reset all attributes
    pub const RESET: &str = "\x1b[0m";
    
    /// Bold/bright
    pub const BOLD: &str = "\x1b[1m";
    
    /// Dim
    pub const DIM: &str = "\x1b[2m";
    
    /// Underline
    pub const UNDERLINE: &str = "\x1b[4m";
    
    /// Foreground colors
    pub mod fg {
        pub const BLACK: &str = "\x1b[30m";
        pub const RED: &str = "\x1b[31m";
        pub const GREEN: &str = "\x1b[32m";
        pub const YELLOW: &str = "\x1b[33m";
        pub const BLUE: &str = "\x1b[34m";
        pub const MAGENTA: &str = "\x1b[35m";
        pub const CYAN: &str = "\x1b[36m";
        pub const WHITE: &str = "\x1b[37m";
        
        /// Bright colors
        pub const BRIGHT_BLACK: &str = "\x1b[90m";
        pub const BRIGHT_RED: &str = "\x1b[91m";
        pub const BRIGHT_GREEN: &str = "\x1b[92m";
        pub const BRIGHT_YELLOW: &str = "\x1b[93m";
        pub const BRIGHT_BLUE: &str = "\x1b[94m";
        pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
        pub const BRIGHT_CYAN: &str = "\x1b[96m";
        pub const BRIGHT_WHITE: &str = "\x1b[97m";
    }
    
    /// Background colors
    pub mod bg {
        pub const BLACK: &str = "\x1b[40m";
        pub const RED: &str = "\x1b[41m";
        pub const GREEN: &str = "\x1b[42m";
        pub const YELLOW: &str = "\x1b[43m";
        pub const BLUE: &str = "\x1b[44m";
        pub const MAGENTA: &str = "\x1b[45m";
        pub const CYAN: &str = "\x1b[46m";
        pub const WHITE: &str = "\x1b[47m";
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_telnet_command_conversion() {
        assert_eq!(TelnetCommand::from_byte(255), Some(TelnetCommand::IAC));
        assert_eq!(TelnetCommand::from_byte(253), Some(TelnetCommand::DO));
        assert_eq!(TelnetCommand::from_byte(251), Some(TelnetCommand::WILL));
        assert_eq!(TelnetCommand::from_byte(100), None);
        
        assert_eq!(TelnetCommand::IAC.to_byte(), 255);
        assert_eq!(TelnetCommand::DO.to_byte(), 253);
    }
    
    #[test]
    fn test_telnet_option_conversion() {
        assert_eq!(TelnetOption::from_byte(1), Some(TelnetOption::Echo));
        assert_eq!(TelnetOption::from_byte(31), Some(TelnetOption::NAWS));
        assert_eq!(TelnetOption::from_byte(69), Some(TelnetOption::MSDP));
        assert_eq!(TelnetOption::from_byte(201), Some(TelnetOption::GMCP));
        assert_eq!(TelnetOption::from_byte(200), None);
        
        assert_eq!(TelnetOption::Echo.to_byte(), 1);
        assert_eq!(TelnetOption::NAWS.to_byte(), 31);
    }
    
    #[test]
    fn test_build_negotiation() {
        let neg = build_negotiation(TelnetCommand::WILL, TelnetOption::Echo);
        assert_eq!(neg, vec![255, 251, 1]);
        
        let neg = build_negotiation(TelnetCommand::DO, TelnetOption::NAWS);
        assert_eq!(neg, vec![255, 253, 31]);
    }
    
    #[test]
    fn test_build_subnegotiation() {
        let data = b"test";
        let subneg = build_subnegotiation(TelnetOption::TerminalType, data);
        assert_eq!(subneg[0], 255); // IAC
        assert_eq!(subneg[1], 250); // SB
        assert_eq!(subneg[2], 24);  // Terminal Type
        assert_eq!(&subneg[3..7], b"test");
        assert_eq!(subneg[7], 255); // IAC
        assert_eq!(subneg[8], 240); // SE
    }
    
    #[test]
    fn test_build_subnegotiation_with_iac() {
        let data = &[255, 100]; // Contains IAC
        let subneg = build_subnegotiation(TelnetOption::MSDP, data);
        // Should have doubled IAC
        assert!(subneg.contains(&255));
        assert_eq!(subneg.iter().filter(|&&b| b == 255).count(), 4); // 2 for frame + 2 for escaped IAC
    }
    
    #[test]
    fn test_parse_window_size() {
        let data = [0, 80, 0, 24]; // 80x24
        assert_eq!(parse_window_size(&data), Some((80, 24)));
        
        let data = [1, 0, 0, 200]; // 256x200
        assert_eq!(parse_window_size(&data), Some((256, 200)));
        
        let data = [0, 80]; // Too short
        assert_eq!(parse_window_size(&data), None);
    }
}

