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

//! Avatar/character data types

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Avatar linkage between account and player entity
/// This is a minimal table - all character data comes from ECS components
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Avatar {
    pub entity_id: Uuid,
    pub account_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_played: Option<chrono::DateTime<chrono::Utc>>,
}

/// Extended avatar information for character selection display
/// This is fetched by joining with component tables
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AvatarInfo {
    pub entity_id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub last_played: Option<chrono::DateTime<chrono::Utc>>,
}

impl AvatarInfo {
    /// Get a display string for the avatar
    pub fn display(&self) -> String {
        format!("{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_info_display() {
        let avatar_info = AvatarInfo {
            entity_id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            name: "Gandalf".to_string(),
            last_played: None,
        };

        let display = avatar_info.display();
        assert!(display.contains("Gandalf"));
    }
}


