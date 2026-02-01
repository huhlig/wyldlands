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
//! - Gateway-to-Server RPC server (gRPC)
//! - Shared type aliases for RPC communication
//! - Utility functions

pub mod gateway;
pub mod utility;

// gRPC generated code
pub mod proto {
    use tonic::transport::Channel;

    tonic::include_proto!("wyldlands.gateway");

    pub use gateway_management_server::GatewayManagement;
    pub use gateway_management_server::GatewayManagementServer;
    pub use session_to_world_server::SessionToWorld;
    pub use session_to_world_server::SessionToWorldServer;
    pub use world_to_session_server::WorldToSession;
    pub use world_to_session_server::WorldToSessionServer;
    pub type GatewayManagementClient = gateway_management_client::GatewayManagementClient<Channel>;
    pub type SessionToWorldClient = session_to_world_client::SessionToWorldClient<Channel>;
    pub type WorldToSessionClient = world_to_session_client::WorldToSessionClient<Channel>;
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
