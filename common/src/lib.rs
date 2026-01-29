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

//! Wyldlands Common Types and Protocols
//!
//! This crate defines shared types and communication protocols used across Wyldlands MUD:
//! - Common data types (Account, Avatar, Session)
//! - Gateway-to-Server RPC protocol (gRPC)
//! - MUD Server Data Protocol (MSDP)

pub mod account;
pub mod character;
pub mod gateway;
pub mod msdp;
pub mod session;
pub mod utility;

// gRPC generated code
pub mod proto {
    tonic::include_proto!("wyldlands.gateway");
}

// Mudnet protocol removed - using gRPC now



#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
