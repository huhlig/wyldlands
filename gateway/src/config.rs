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

use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_env_field::EnvField;
use std::net::{AddrParseError, IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Arguments {
    #[arg(
        short = 'c',
        long = "config",
        help = "Path to configuration file",
        default_value = "gateway/config.yaml"
    )]
    pub config_file: String,

    #[arg(
        short = 'e',
        long = "env",
        help = "Path to environment file",
        default_value = "gateway/.env"
    )]
    pub env_file: Option<String>,

    #[arg(
        short = 'w',
        long = "websocket",
        help = "Enable websocket server",
        default_value = "true"
    )]
    pub websocket: bool,

    #[arg(
        short = 't',
        long = "telnet",
        help = "Enable telnet server",
        default_value = "true"
    )]
    pub telnet: bool,
}

impl Default for Arguments {
    fn default() -> Self {
        Self {
            config_file: "config.yaml".to_string(),
            env_file: Some(".env".to_string()),
            websocket: false,
            telnet: false,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Configuration {
    #[serde(default)]
    pub server: WorldServerConfig,

    #[serde(default)]
    pub database: DatabaseConfig,

    pub telnet: Option<TelnetConfig>,
    pub websocket: Option<WebsocketConfig>,
    pub grpc: Option<GrpcConfig>,
}

impl Configuration {
    pub fn load(path: &str) -> Result<Self, String> {
        tracing::debug!("Loading configuration from file: {}", path);
        let file =
            std::fs::File::open(path).map_err(|e| format!("Failed to open config file: {}", e))?;

        let conf = serde_yaml::from_reader(file)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;

        Ok(conf)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub url: EnvField<String>,

    #[serde(default)]
    pub username: EnvField<String>,

    #[serde(default)]
    pub password: EnvField<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorldServerConfig {
    pub addr: EnvField<WorldServerAddress>,

    /// Authentication key for gateway-to-server communication
    pub auth_key: EnvField<WorldServerAuthKey>,

    /// Heartbeat interval in seconds (default: 30)
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: u64,

    /// Reconnection interval in seconds (default: 5)
    #[serde(default = "default_reconnect_interval")]
    pub reconnect_interval: u64,
}

fn default_heartbeat_interval() -> u64 {
    30
}

fn default_reconnect_interval() -> u64 {
    5
}

impl Default for WorldServerConfig {
    fn default() -> Self {
        WorldServerConfig {
            addr: Default::default(),
            auth_key: Default::default(),
            heartbeat_interval: default_heartbeat_interval(),
            reconnect_interval: default_reconnect_interval(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorldServerAddress(String);

impl WorldServerAddress {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn to_addrs(&self) -> std::io::Result<impl Iterator<Item = SocketAddr>> {
        std::net::ToSocketAddrs::to_socket_addrs(self.0.as_str())
    }
}

impl FromStr for WorldServerAddress {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl Default for WorldServerAddress {
    fn default() -> Self {
        Self(String::from("localhost:6006"))
    }
}

impl std::fmt::Display for WorldServerAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({})",
            self.0,
            std::net::ToSocketAddrs::to_socket_addrs(self.0.as_str())
                .map_err(|_| std::fmt::Error::default())?
                .map(|addr| addr.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorldServerAuthKey(String);

impl WorldServerAuthKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for WorldServerAuthKey {

    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl FromStr for WorldServerAuthKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl Default for WorldServerAuthKey {
    fn default() -> Self {
        Self(String::from("default-secret-key-change-in-production"))
    }
}

impl std::fmt::Display for WorldServerAuthKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'", self.0)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TelnetConfig {
    pub addr: EnvField<TelnetBinding>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelnetBinding(SocketAddr);

impl TelnetBinding {
    pub fn to_addr(&self) -> SocketAddr {
        self.0
    }
    pub fn to_ip(&self) -> IpAddr {
        self.0.ip()
    }
    pub fn to_port(&self) -> u16 {
        self.0.port()
    }
}

impl FromStr for TelnetBinding {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(SocketAddr::from_str(s)?))
    }
}

impl Default for TelnetBinding {
    fn default() -> Self {
        Self(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(0, 0, 0, 0),
            4000,
        )))
    }
}

impl std::fmt::Display for TelnetBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WebsocketConfig {
    pub addr: EnvField<WebsocketBinding>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebsocketBinding(SocketAddr);

impl WebsocketBinding {
    pub fn to_addr(&self) -> SocketAddr {
        self.0
    }
    pub fn to_ip(&self) -> IpAddr {
        self.0.ip()
    }
    pub fn to_port(&self) -> u16 {
        self.0.port()
    }
}

impl FromStr for WebsocketBinding {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(SocketAddr::from_str(s)?))
    }
}

impl Default for WebsocketBinding {
    fn default() -> Self {
        Self(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(0, 0, 0, 0),
            8080,
        )))
    }
}

impl std::fmt::Display for WebsocketBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GrpcConfig {
    pub addr: EnvField<GrpcBinding>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrpcBinding(SocketAddr);

impl GrpcBinding {
    pub fn to_addr(&self) -> SocketAddr {
        self.0
    }
    pub fn to_ip(&self) -> IpAddr {
        self.0.ip()
    }
    pub fn to_port(&self) -> u16 {
        self.0.port()
    }
}

impl FromStr for GrpcBinding {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(SocketAddr::from_str(s)?))
    }
}

impl Default for GrpcBinding {
    fn default() -> Self {
        Self(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(0, 0, 0, 0),
            5001,
        )))
    }
}

impl std::fmt::Display for GrpcBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::NamedTempFile;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_world_server_config_default() {
        let config = WorldServerConfig::default();
        assert_eq!(config.addr.as_str(), "localhost:6006");
        // localhost may resolve to both IPv6 and IPv4, so just check that it contains the IPv4 address
        let addrs: Vec<SocketAddr> = config.addr.to_addrs().unwrap().collect();
        assert!(addrs.contains(&SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(127, 0, 0, 1),
            6006
        ))));
    }

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.url.as_str(), "");
        assert_eq!(config.username.as_str(), "");
        assert_eq!(config.password.as_str(), "");
    }

    #[test]
    fn test_websocket_config_default() {
        let config = WebsocketConfig::default();
        assert_eq!(
            config.addr.to_addr(),
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 8080))
        );
        assert_eq!(config.addr.to_ip(), Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(config.addr.to_port(), 8080);
    }

    #[test]
    fn test_telnet_config_default() {
        let config = TelnetConfig::default();
        assert_eq!(
            config.addr.to_addr(),
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 4000))
        );
        assert_eq!(config.addr.to_ip(), Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(config.addr.to_port(), 4000);
    }

    #[test]
    fn test_configuration_new_from_file() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
        writeln!(
            file,
            r#"
server:
  addr: 127.0.0.1:7000
  auth_key: test-auth-key
database:
  url: "postgres://localhost/db"
telnet:
  addr: 127.0.0.1:4001
websocket:
  addr: 127.0.0.1:8081
"#
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        // Ensure no environment variables interfere - clear all possible env vars
        unsafe {
            std::env::remove_var("WYLDLANDS_SERVER_PORT");
            std::env::remove_var("WYLDLANDS_SERVER_ADDR");
            std::env::remove_var("WYLDLANDS_DATABASE_URL");
            std::env::remove_var("WYLDLANDS_TELNET_ADDR");
            std::env::remove_var("WYLDLANDS_TELNET_PORT");
            std::env::remove_var("WYLDLANDS_WEBSOCKET_ADDR");
            std::env::remove_var("WYLDLANDS_WEBSOCKET_PORT");
        }

        // Don't load any .env file for this test
        let config = Configuration::load(path).unwrap();

        assert_eq!(
            config
                .server
                .addr
                .to_addrs()
                .unwrap()
                .next()
                .unwrap()
                .port(),
            7000
        );
        assert_eq!(config.database.url.as_str(), "postgres://localhost/db");
        assert_eq!(config.telnet.unwrap().addr.to_port(), 4001);
        assert_eq!(config.websocket.unwrap().addr.to_port(), 8081);
    }

    #[test]
    fn test_configuration_env_override() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
        writeln!(
            file,
            r#"
server:
  addr: "${{WYLDLANDS_SERVER_ADDR:-127.0.0.1:7000}}"
  auth_key: test-auth-key
database:
  url: "postgres://localhost/db"
"#
        )
        .unwrap();

        let path = file.path().to_str().unwrap();

        unsafe {
            std::env::set_var("WYLDLANDS_SERVER_ADDR", "127.0.0.1:9000");
        }

        let config = Configuration::load(path).unwrap();

        unsafe {
            std::env::remove_var("WYLDLANDS_SERVER_ADDR");
        }

        assert_eq!(
            config
                .server
                .addr
                .to_addrs()
                .unwrap()
                .next()
                .unwrap()
                .port(),
            9000
        );
    }

    #[test]
    fn test_configuration_env_override_with_hostname() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
        writeln!(
            file,
            r#"
server:
  addr: "${{WYLDLANDS_SERVER_ADDR:-127.0.0.1:7000}}"
  auth_key: test-auth-key
database:
  url: "postgres://localhost/db"
"#
        )
        .unwrap();

        let path = file.path().to_str().unwrap();

        unsafe {
            std::env::set_var("WYLDLANDS_SERVER_ADDR", "127.0.0.1:9000");
        }

        let config = Configuration::load(path).unwrap();

        unsafe {
            std::env::remove_var("WYLDLANDS_SERVER_ADDR");
        }

        let addr = config.server.addr.to_addrs().unwrap().next().unwrap();
        assert_eq!(
            addr,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000)
        );
        assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(addr.port(), 9000);
    }
}


