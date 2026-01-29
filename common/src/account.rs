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

//! Account data types

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Account role for access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "account_role", rename_all = "lowercase")]
pub enum AccountRole {
    Player,
    Storyteller,
    Builder,
    Admin,
}

impl AccountRole {
    /// Check if this role has at least the specified role level
    pub fn has_permission(&self, required: AccountRole) -> bool {
        let self_level = self.level();
        let required_level = required.level();
        self_level >= required_level
    }
    
    /// Get numeric level for role comparison
    fn level(&self) -> u8 {
        match self {
            AccountRole::Player => 0,
            AccountRole::Storyteller => 1,
            AccountRole::Builder => 2,
            AccountRole::Admin => 3,
        }
    }
}

impl std::fmt::Display for AccountRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountRole::Player => write!(f, "Player"),
            AccountRole::Storyteller => write!(f, "Storyteller"),
            AccountRole::Builder => write!(f, "Builder"),
            AccountRole::Admin => write!(f, "Admin"),
        }
    }
}

/// Account information (password is never included for security)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Account {
    pub id: Uuid,
    pub login: String,
    pub display: String,
    pub timezone: Option<String>,
    pub discord: Option<String>,
    pub email: Option<String>,
    pub rating: i32,
    pub active: bool,
    pub role: AccountRole,
}

impl Account {
    /// Create a new account (for testing only - use AuthManager in production)
    pub fn new(login: String, display: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            login,
            display,
            timezone: None,
            discord: None,
            email: None,
            rating: 0,
            active: true,
            role: AccountRole::Player,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_account_creation() {
        let account = Account::new(
            "testuser".to_string(),
            "Test User".to_string(),
        );
        
        assert_eq!(account.login, "testuser");
        assert_eq!(account.display, "Test User");
        assert!(account.active);
        assert_eq!(account.role, AccountRole::Player);
    }
    
    #[test]
    fn test_account_serialization() {
        let account = Account::new(
            "testuser".to_string(),
            "Test User".to_string(),
        );
        
        let json = serde_json::to_string(&account).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("Test User"));
        // Password field no longer exists in Account struct
    }
}

