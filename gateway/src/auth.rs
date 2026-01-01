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

//! Authentication and account management

use crate::avatar::{Avatar, AvatarInfo};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use wyldlands_common::account::Account;
use wyldlands_common::character::StartingLocation;

/// Authentication manager
pub struct AuthManager {
    pool: PgPool,
}

/// Account creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountRequest {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub email: Option<String>,
}

impl CreateAccountRequest {
    /// Validate the account creation request
    pub fn validate(&self) -> Result<(), String> {
        // Validate username
        if self.username.is_empty() {
            return Err("Username cannot be empty".to_string());
        }
        if self.username.len() < 3 {
            return Err("Username must be at least 3 characters".to_string());
        }
        if self.username.len() > 20 {
            return Err("Username must be at most 20 characters".to_string());
        }
        if !self.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("Username can only contain letters, numbers, and underscores".to_string());
        }
        
        // Validate display name
        if self.display_name.is_empty() {
            return Err("Display name cannot be empty".to_string());
        }
        if self.display_name.len() > 50 {
            return Err("Display name must be at most 50 characters".to_string());
        }
        
        // Validate password
        if self.password.is_empty() {
            return Err("Password cannot be empty".to_string());
        }
        if self.password.len() < 6 {
            return Err("Password must be at least 6 characters".to_string());
        }
        if self.password.len() > 100 {
            return Err("Password must be at most 100 characters".to_string());
        }
        
        // Validate email if provided
        if let Some(email) = &self.email {
            if !email.is_empty() && !email.contains('@') {
                return Err("Invalid email address".to_string());
            }
        }
        
        Ok(())
    }
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Authenticate a user with username and password
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<Account, String> {
        // Fetch account and verify password using pgcrypto's crypt()
        // This compares the provided password against the stored bcrypt hash
        // Note: password field is NOT selected for security
        let account: Option<Account> = sqlx::query_as(
            "SELECT id, login, display, timezone, discord, email, active, rating, admin
             FROM wyldlands.accounts
             WHERE LOWER(login) = LOWER($1)
             AND password = crypt($2, password)"
        )
        .bind(username)
        .bind(password)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        let account = account.ok_or_else(|| "Invalid username or password".to_string())?;
        
        // Check if account is active
        if !account.active {
            return Err("Account is disabled".to_string());
        }
        
        Ok(account)
    }

    /// Get account by ID
    pub async fn get_account_by_id(&self, account_id: Uuid) -> Result<Account, String> {
        let account: Option<Account> = sqlx::query_as(
            "SELECT id, login, display, timezone, discord, email, active, rating, admin
             FROM wyldlands.accounts
             WHERE id = $1"
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        account.ok_or_else(|| "Account not found".to_string())
    }

    /// Get all avatars for an account with display information
    /// Joins with component tables to get character details
    pub async fn get_avatars(&self, account_id: Uuid) -> Result<Vec<AvatarInfo>, String> {
        sqlx::query_as(
            "SELECT 
                ea.entity_id,
                ea.account_id,
                COALESCE(n.display, 'Unnamed') as name,
                ea.last_played
             FROM wyldlands.entity_avatars ea
             LEFT JOIN wyldlands.entity_name n ON ea.entity_id = n.entity_id
             WHERE ea.account_id = $1
             ORDER BY ea.last_played DESC NULLS LAST, ea.created_at DESC"
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch avatars: {}", e))
    }
    
    /// Get a specific avatar by entity ID
    pub async fn get_avatar(&self, entity_id: Uuid) -> Result<Avatar, String> {
        sqlx::query_as(
            "SELECT entity_id, account_id, created_at, last_played
             FROM wyldlands.entity_avatars
             WHERE entity_id = $1"
        )
        .bind(entity_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Avatar not found: {}", e))
    }
    
    /// Create avatar linkage (entity must already exist from server)
    pub async fn link_avatar(
        &self,
        account_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Avatar, String> {
        let avatar: Avatar = sqlx::query_as(
            "INSERT INTO wyldlands.entity_avatars 
             (entity_id, account_id, created_at)
             VALUES ($1, $2, NOW())
             RETURNING entity_id, account_id, created_at, last_played"
        )
        .bind(entity_id)
        .bind(account_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to link avatar: {}", e))?;
        
        Ok(avatar)
    }
    
    /// Update avatar's last played timestamp
    pub async fn update_last_played(&self, entity_id: Uuid) -> Result<(), String> {
        sqlx::query(
            "UPDATE wyldlands.entity_avatars SET last_played = NOW() WHERE entity_id = $1"
        )
        .bind(entity_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update last played: {}", e))?;
        
        Ok(())
    }
    
    /// Delete an avatar (also deletes the entity via CASCADE)
    pub async fn delete_avatar(&self, entity_id: Uuid, account_id: Uuid) -> Result<(), String> {
        let result = sqlx::query(
            "DELETE FROM wyldlands.entity_avatars WHERE entity_id = $1 AND account_id = $2"
        )
        .bind(entity_id)
        .bind(account_id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to delete avatar: {}", e))?;
        
        if result.rows_affected() == 0 {
            return Err("Avatar not found or you don't own it".to_string());
        }
        
        Ok(())
    }
    
    /// Create a new account
    pub async fn create_account(&self, request: CreateAccountRequest) -> Result<Account, String> {
        // Check if account creation is enabled
        let setting: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM wyldlands.settings WHERE key = 'account.creation_enabled'"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        let creation_enabled = setting
            .map(|(value,)| value.to_lowercase() == "true")
            .unwrap_or(true); // Default to true if setting doesn't exist
        
        if !creation_enabled {
            return Err("Account creation is currently disabled".to_string());
        }
        
        // Validate the request
        request.validate()?;
        
        // Check if username already exists
        let existing: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM wyldlands.accounts WHERE LOWER(login) = LOWER($1)"
        )
        .bind(&request.username)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        if existing.0 > 0 {
            return Err("Username already exists".to_string());
        }
        
        // Check if email already exists (if provided)
        if let Some(email) = &request.email {
            if !email.is_empty() {
                let existing: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM wyldlands.accounts WHERE LOWER(email) = LOWER($1)"
                )
                .bind(email)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| format!("Database error: {}", e))?;
                
                if existing.0 > 0 {
                    return Err("Email already registered".to_string());
                }
            }
        }
        
        // Hash password using PostgreSQL's crypt() function with bcrypt
        // This uses pgcrypto extension which is already enabled
        // Note: password field is NOT returned for security
        let account_id = Uuid::new_v4();
        let account: Account = sqlx::query_as(
            "INSERT INTO wyldlands.accounts
             (id, login, display, password, email, active, admin)
             VALUES ($1, $2, $3, crypt($4, gen_salt('bf')), $5, true, false)
             RETURNING id, login, display, timezone, discord, email, rating, active, admin"
        )
        .bind(account_id)
        .bind(&request.username)
        .bind(&request.display_name)
        .bind(&request.password)
        .bind(&request.email)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to create account: {}", e))?;
        
        Ok(account)
    }
    
    /// Check if a username is available
    pub async fn is_username_available(&self, username: &str) -> Result<bool, String> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM wyldlands.accounts WHERE LOWER(login) = LOWER($1)"
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        Ok(count.0 == 0)
    }
    
    /// Get all available starting locations
    pub async fn get_starting_locations(&self) -> Result<Vec<StartingLocation>, String> {
        let locations: Vec<StartingLocation> = sqlx::query_as(
            "SELECT id, name, description, room_id, enabled, sort_order
             FROM wyldlands.starting_locations
             WHERE enabled = true
             ORDER BY sort_order ASC, name ASC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch starting locations: {}", e))?;
        
        Ok(locations)
    }
    
    /// Get a specific starting location by ID
    pub async fn get_starting_location(&self, location_id: &str) -> Result<StartingLocation, String> {
        let location: StartingLocation = sqlx::query_as(
            "SELECT id, name, description, room_id, enabled, sort_order
             FROM wyldlands.starting_locations
             WHERE id = $1 AND enabled = true"
        )
        .bind(location_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Starting location not found: {}", e))?;
        
        Ok(location)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_account_type_available() {
        // Just verify the types are accessible
        let _account_type: Option<Account> = None;
        let _avatar_type: Option<Avatar> = None;
        let _avatar_info_type: Option<AvatarInfo> = None;
    }
}

