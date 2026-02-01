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

//! Telnet server handler for the Wyldlands Gateway
//!
//! This module provides telnet server support using the termionix library,
//! including support for:
//! - Basic telnet server negotiation
//! - MCCP (MUD Client Compression Protocol)
//! - MSDP (MUD Server Data Protocol)
//! - GMCP (Generic MUD Communication Protocol)
//! - NAWS (Negotiate About Window Size)
//! - ANSI color codes

// Termionix-based telnet implementation
mod adapter;
mod handler;
mod server;

pub use self::server::TermionixTelnetServer;

#[cfg(test)]
mod tests {
    // Tests for Termionix-based telnet implementation
    // Note: Most testing is done through integration tests

    #[test]
    fn test_module_structure() {
        // Basic test to ensure module compiles
        assert!(true);
    }
}
