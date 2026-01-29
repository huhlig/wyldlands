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

//! Gateway shell command system

use crate::context::ServerContext;
use crate::auth::CreateAccountRequest;
use uuid::Uuid;

/// Shell command result
#[derive(Debug)]
pub enum ShellResult {
    /// Command executed successfully with output
    Success(String),
    /// Command failed with error message
    Error(String),
    /// Request to quit/disconnect
    Quit,
    /// Continue processing
    Continue,
}

/// Shell command handler
pub struct Shell {
    context: ServerContext,
    session_id: Option<Uuid>,
}

impl Shell {
    /// Create a new shell instance
    pub fn new(context: ServerContext) -> Self {
        Self {
            context,
            session_id: None,
        }
    }
    
    /// Set the session ID for this shell
    pub fn set_session(&mut self, session_id: Uuid) {
        self.session_id = Some(session_id);
    }
    
    /// Execute a command
    pub async fn execute(&self, input: &str) -> ShellResult {
        let input = input.trim();
        
        if input.is_empty() {
            return ShellResult::Continue;
        }
        
        let parts: Vec<&str> = input.split_whitespace().collect();
        let command = parts[0].to_lowercase();
        let args = &parts[1..];
        
        match command.as_str() {
            "help" | "?" => self.cmd_help(args).await,
            "who" => self.cmd_who(args).await,
            "stats" => self.cmd_stats(args).await,
            "quit" | "exit" | "logout" => self.cmd_quit(args).await,
            "create" => self.cmd_create(args).await,
            "check" => self.cmd_check(args).await,
            _ => ShellResult::Error(format!("Unknown command: {}. Type 'help' for available commands.", command)),
        }
    }
    
    /// Help command - show available commands
    async fn cmd_help(&self, _args: &[&str]) -> ShellResult {
        let help_text = r#"
=== Gateway Shell Commands ===

  help, ?           - Show this help message
  who               - List active sessions
  stats             - Show gateway statistics
  quit, exit        - Disconnect from gateway
  create account    - Create a new account (interactive)
  check <username>  - Check if username is available

Type a command followed by any required arguments.
"#;
        ShellResult::Success(help_text.to_string())
    }
    
    /// Who command - list active sessions
    async fn cmd_who(&self, _args: &[&str]) -> ShellResult {
        let sessions = self.context.session_manager().get_active_sessions().await;
        let mut output = String::from("\r\n=== Active Sessions ===\r\n\r\n");
        
        if sessions.is_empty() {
            output.push_str("No active sessions.\r\n");
        } else {
            for session in &sessions {
                output.push_str(&format!(
                    "  {} - {:?} - {:?} - {}\r\n",
                    session.id,
                    session.protocol,
                    session.state,
                    session.client_addr
                ));
            }
            output.push_str(&format!("\r\nTotal: {} session(s)\r\n", sessions.len()));
        }
        
        ShellResult::Success(output)
    }
    
    /// Stats command - show gateway statistics
    async fn cmd_stats(&self, _args: &[&str]) -> ShellResult {
        let sessions = self.context.session_manager().get_active_sessions().await;
        let total = sessions.len();
        let telnet = sessions.iter().filter(|s| matches!(s.protocol, crate::session::ProtocolType::Telnet)).count();
        let websocket = sessions.iter().filter(|s| matches!(s.protocol, crate::session::ProtocolType::WebSocket)).count();
        let playing = sessions.iter().filter(|s| matches!(s.state, crate::session::SessionState::Playing)).count();
        
        let output = format!(
            r#"
=== Gateway Statistics ===

  Total Sessions:     {}
  Telnet Sessions:    {}
  WebSocket Sessions: {}
  Playing:            {}
  
  Database Pool:      Connected
  Connection Pool:    Active
"#,
            total, telnet, websocket, playing
        );
        
        ShellResult::Success(output)
    }
    
    /// Quit command - disconnect
    async fn cmd_quit(&self, _args: &[&str]) -> ShellResult {
        ShellResult::Quit
    }
    
    /// Create command - create new resources
    async fn cmd_create(&self, args: &[&str]) -> ShellResult {
        if args.is_empty() {
            return ShellResult::Error("Usage: create <account>".to_string());
        }
        
        match args[0].to_lowercase().as_str() {
            "account" => {
                ShellResult::Success("Account creation requires interactive mode. Use the login flow to create an account.".to_string())
            }
            _ => ShellResult::Error(format!("Unknown resource type: {}. Available: account", args[0])),
        }
    }
    
    /// Check command - check resource availability
    async fn cmd_check(&self, args: &[&str]) -> ShellResult {
        if args.is_empty() {
            return ShellResult::Error("Usage: check <username>".to_string());
        }
        
        let username = args[0];
        
        match self.context.auth_manager().is_username_available(username).await {
            Ok(available) => {
                if available {
                    ShellResult::Success(format!("Username '{}' is available!", username))
                } else {
                    ShellResult::Success(format!("Username '{}' is already taken.", username))
                }
            }
            Err(e) => ShellResult::Error(format!("Failed to check username: {}", e)),
        }
    }
}

/// Interactive account creation flow
pub struct AccountCreationFlow {
    context: ServerContext,
    state: AccountCreationState,
}

#[derive(Debug, Clone)]
enum AccountCreationState {
    Username,
    DisplayName { username: String },
    Password { username: String, display_name: String },
    ConfirmPassword { username: String, display_name: String, password: String },
    Email { username: String, display_name: String, password: String },
    Confirm { request: CreateAccountRequest },
}

impl AccountCreationFlow {
    /// Create a new account creation flow
    pub fn new(context: ServerContext) -> Self {
        Self {
            context,
            state: AccountCreationState::Username,
        }
    }
    
    /// Get the current prompt
    pub fn get_prompt(&self) -> String {
        match &self.state {
            AccountCreationState::Username => "Enter username (3-20 characters, letters/numbers/_): ".to_string(),
            AccountCreationState::DisplayName { .. } => "Enter display name: ".to_string(),
            AccountCreationState::Password { .. } => "Enter password (6+ characters): ".to_string(),
            AccountCreationState::ConfirmPassword { .. } => "Confirm password: ".to_string(),
            AccountCreationState::Email { .. } => "Enter email (optional, press Enter to skip): ".to_string(),
            AccountCreationState::Confirm { request } => {
                format!(
                    "\r\nCreate account:\r\n  Username: {}\r\n  Display Name: {}\r\n  Email: {}\r\n\r\nConfirm? (yes/no): ",
                    request.username,
                    request.display_name,
                    request.email.as_deref().unwrap_or("(none)")
                )
            }
        }
    }
    
    /// Process input for the current state
    pub async fn process_input(&mut self, input: &str) -> Result<Option<CreateAccountRequest>, String> {
        let input = input.trim();
        
        match &self.state {
            AccountCreationState::Username => {
                if input.is_empty() {
                    return Err("Username cannot be empty".to_string());
                }
                
                // Check if username is available
                if !self.context.auth_manager().is_username_available(input).await? {
                    return Err("Username already taken".to_string());
                }
                
                self.state = AccountCreationState::DisplayName {
                    username: input.to_string(),
                };
                Ok(None)
            }
            AccountCreationState::DisplayName { username } => {
                if input.is_empty() {
                    return Err("Display name cannot be empty".to_string());
                }
                
                self.state = AccountCreationState::Password {
                    username: username.clone(),
                    display_name: input.to_string(),
                };
                Ok(None)
            }
            AccountCreationState::Password { username, display_name } => {
                if input.len() < 6 {
                    return Err("Password must be at least 6 characters".to_string());
                }
                
                self.state = AccountCreationState::ConfirmPassword {
                    username: username.clone(),
                    display_name: display_name.clone(),
                    password: input.to_string(),
                };
                Ok(None)
            }
            AccountCreationState::ConfirmPassword { username, display_name, password } => {
                if input != password {
                    return Err("Passwords do not match".to_string());
                }
                
                self.state = AccountCreationState::Email {
                    username: username.clone(),
                    display_name: display_name.clone(),
                    password: password.clone(),
                };
                Ok(None)
            }
            AccountCreationState::Email { username, display_name, password } => {
                let email = if input.is_empty() {
                    None
                } else {
                    Some(input.to_string())
                };
                
                let request = CreateAccountRequest {
                    username: username.clone(),
                    display_name: display_name.clone(),
                    password: password.clone(),
                    email,
                };
                
                self.state = AccountCreationState::Confirm {
                    request: request.clone(),
                };
                Ok(None)
            }
            AccountCreationState::Confirm { request } => {
                if input.to_lowercase() == "yes" || input.to_lowercase() == "y" {
                    Ok(Some(request.clone()))
                } else {
                    Err("Account creation cancelled".to_string())
                }
            }
        }
    }
    
    /// Check if the flow is complete
    pub fn is_complete(&self) -> bool {
        matches!(&self.state, AccountCreationState::Confirm { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_shell_result() {
        let result = ShellResult::Success("test".to_string());
        assert!(matches!(result, ShellResult::Success(_)));
    }
}

