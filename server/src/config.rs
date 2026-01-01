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
        default_value = "server/config.yaml"
    )]
    pub config_file: String,

    #[arg(
        short = 'e',
        long = "env",
        help = "Path to environment file",
        default_value = "server/.env"
    )]
    pub env_file: Option<String>,
}

impl Default for Arguments {
    fn default() -> Self {
        Self {
            config_file: "config.yaml".to_string(),
            env_file: Some(".env".to_string()),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Configuration {
    pub database: DatabaseConfig,
    pub listener: GatewayListenerConfig,
}

impl Configuration {
    pub fn load(path: &str) -> Result<Configuration, String> {
        let conf = serde_yaml::from_reader(
            std::fs::File::open(path).map_err(|e| format!("Failed to open config file: {}", e))?,
        )
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

        Ok(conf)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: EnvField<String>,
    pub username: EnvField<String>,
    pub password: EnvField<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayListenerConfig {
    pub addr: EnvField<GatewayListenerBinding>,

    /// Authentication key for gateway-to-server communication
    pub auth_key: EnvField<GatewayListenerAuthKey>,
}

impl Default for GatewayListenerConfig {
    fn default() -> Self {
        Self {
            addr: Default::default(),
            auth_key: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayListenerBinding(SocketAddr);

impl GatewayListenerBinding {
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

impl FromStr for GatewayListenerBinding {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        tracing::debug!("Parsing gateway listener binding from string: {}", s);
        Ok(Self(SocketAddr::from_str(s)?))
    }
}

impl Default for GatewayListenerBinding {
    fn default() -> Self {
        Self(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(0, 0, 0, 0),
            6006,
        )))
    }
}

impl std::fmt::Display for GatewayListenerBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayListenerAuthKey(String);

impl GatewayListenerAuthKey {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for GatewayListenerAuthKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl FromStr for GatewayListenerAuthKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl Default for GatewayListenerAuthKey {
    fn default() -> Self {
        Self(String::from("default-secret-key-change-in-production"))
    }
}

impl std::fmt::Display for GatewayListenerAuthKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_arguments_default() {
        let args = Arguments::default();
        assert_eq!(args.config_file, "config.yaml");
        assert_eq!(args.env_file, Some(".env".to_string()));
    }

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.url.into_inner(), "");
        assert_eq!(config.username.into_inner(), "");
        assert_eq!(config.password.into_inner(), "");
    }

    #[test]
    fn test_gateway_listener_config_default() {
        let config = GatewayListenerConfig::default();
        assert_eq!(config.addr.to_ip(), IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
        assert_eq!(config.addr.to_port(), 6006);
    }

    #[test]
    fn test_configuration_default() {
        let config = Configuration::default();
        assert_eq!(
            config.listener.addr.to_ip(),
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
        );
        assert_eq!(config.listener.addr.to_port(), 6006);
        assert_eq!(config.database.url.into_inner(), "");
    }

    #[test]
    fn test_configuration_load_defaults() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // Clear environment variables that might interfere
        unsafe {
            std::env::remove_var("WYLDLANDS_LISTENER_ADDR");
            std::env::remove_var("WYLDLANDS_LISTENER_PORT");
            std::env::remove_var("WYLDLANDS_DATABASE_URL");
        }
        // Test loading with a non-existent file should return error
        let result = Configuration::load("non_existent.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_configuration_load_from_file() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // Clear environment variables that might interfere
        unsafe {
            std::env::remove_var("WYLDLANDS_LISTENER_ADDR");
            std::env::remove_var("WYLDLANDS_LISTENER_PORT");
            std::env::remove_var("WYLDLANDS_DATABASE_URL");
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("config.yaml");
        std::fs::write(
            &file_path,
            "listener:\r\n  addr: \"192.168.1.1:7000\"\n  auth_key: \"test-key\"\ndatabase:\n  url: \"postgres://localhost/db\"\n  username: \"\"\n  password: \"\""
        ).unwrap();

        let path = file_path.to_str().unwrap();
        let config = Configuration::load(path).unwrap();

        assert_eq!(
            config.listener.addr.to_addr(),
            SocketAddr::from_str("192.168.1.1:7000").unwrap(),
        );
        assert_eq!(
            config.listener.addr.to_ip(),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))
        );
        assert_eq!(config.listener.addr.to_port(), 7000);
        assert_eq!(config.database.url.into_inner(), "postgres://localhost/db");
    }

    #[test]
    #[ignore = "Environment variable override functionality not yet implemented"]
    fn test_configuration_load_with_env_overrides() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("config.yaml");
        std::fs::write(&file_path, "listener:\n  addr: \"0.0.0.0:7000\"\ndatabase:\n  url: \"postgres://localhost/db\"\n  username: \"\"\n  password: \"\"").unwrap();

        let path = file_path.to_str().unwrap();

        // Set environment variables to override
        unsafe {
            std::env::set_var("WYLDLANDS_LISTENER_ADDR", "192.168.0.1:8000");
            std::env::set_var("WYLDLANDS_DATABASE_URL", "postgres://env/db");
        }

        let config = Configuration::load(path).unwrap();

        // Clean up environment variables
        unsafe {
            std::env::remove_var("WYLDLANDS_LISTENER_ADDR");
            std::env::remove_var("WYLDLANDS_DATABASE_URL");
        }

        assert_eq!(
            config.listener.addr.to_ip(),
            IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))
        );
        assert_eq!(config.listener.addr.to_port(), 8000);
        assert_eq!(config.database.url.into_inner(), "postgres://env/db");
    }
}
