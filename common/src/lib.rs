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

//! Wyldlands Common Types and Protocols
//!
//! This crate defines shared types and communication protocols used across Wyldlands MUD:
//! - Common data types (Account, Avatar, Session)
//! - Gateway-to-Server RPC protocol
//! - MUD Server Data Protocol (MSDP)
//! - Legacy Mudnet protocol

pub mod account;
pub mod character;
pub mod gateway;
pub mod msdp;
pub mod session;
pub mod utility;

use std::collections::BTreeMap;
use std::net::SocketAddr;
use tarpc::serde::{Deserialize, Serialize};

#[tarpc::service]
pub trait Mudnet {
    async fn message(addr: SocketAddr, str: String);
    async fn status(str: String);
    async fn mud_server_data(table: MudServerDataTable);
    async fn mud_server_status(status: MudServerStatus);
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MudServerDataValue {
    String(String),
    Array(MudServerDataArray),
    Table(MudServerDataTable),
}

///
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MudServerDataArray(Vec<MudServerDataValue>);

///
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MudServerDataTable(BTreeMap<String, MudServerDataValue>);

///
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MudServerStatus(BTreeMap<String, Vec<String>>);



#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
