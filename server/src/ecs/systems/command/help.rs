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

//! Help system for providing detailed information about commands, skills, talents, etc.

use crate::ecs::context::WorldContext;
use crate::ecs::EcsEntity;
use wyldlands_common::account::AccountRole;
use std::sync::Arc;
use super::CommandResult;

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "help_category", rename_all = "PascalCase")]
pub enum HelpCategory {
    Command,
    Skill,
    Talent,
    Spell,
    Combat,
    Building,
    Social,
    System,
    Lore,
    General,
}

#[derive(Debug, Clone)]
pub struct HelpTopic {
    pub keyword: String,
    pub category: HelpCategory,
    pub title: String,
    pub content: String,
    pub syntax: Option<String>,
    pub examples: Option<String>,
    pub see_also: Vec<String>,
    pub min_role: AccountRole,
}

/// Get help topic from database
fn get_help_topic(context: Arc<WorldContext>, keyword: &str, user_role: AccountRole) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<HelpTopic>> + Send + '_>> {
    Box::pin(async move {
    let keyword_lower = keyword.to_lowercase();
    let db = context.persistence().database();
    
    // First try to find the topic directly
    let result = sqlx::query_as::<_, (String, HelpCategory, String, String, Option<String>, Option<String>, Vec<String>, AccountRole)>(
        "SELECT keyword, category, title, content, syntax, examples, see_also, min_role
         FROM wyldlands.help_topics
         WHERE keyword = $1"
    )
    .bind(&keyword_lower)
    .fetch_optional(db)
    .await;

    if let Ok(Some(row)) = result {
        let (keyword, category, title, content, syntax, examples, see_also, min_role) = row;
        
        // Check if user has permission to view this help
        if !user_role.has_permission(min_role) {
            return None;
        }
        
        return Some(HelpTopic {
            keyword,
            category,
            title,
            content,
            syntax,
            examples,
            see_also,
            min_role,
        });
    }

    // If not found, try to find an alias
    let alias_result = sqlx::query_scalar::<_, String>(
        "SELECT keyword FROM wyldlands.help_aliases WHERE alias = $1"
    )
    .bind(&keyword_lower)
    .fetch_optional(db)
    .await;

    if let Ok(Some(target_keyword)) = alias_result {
        // Recursively get the actual help topic
        return get_help_topic(context, &target_keyword, user_role).await;
    }

    None
    })
}

/// Format help topic for display
fn format_help_topic(topic: &HelpTopic) -> String {
    let mut output = String::new();
    
    output.push_str("\r\n");
    output.push_str("╔══════════════════════════════════════════════════════════════╗\r\n");
    output.push_str(&format!("║ {:^60} ║\r\n", topic.title));
    output.push_str("╚══════════════════════════════════════════════════════════════╝\r\n");
    output.push_str("\r\n");
    
    // Category
    output.push_str(&format!("Category: {:?}\r\n\r\n", topic.category));
    
    // Content
    for line in topic.content.lines() {
        output.push_str(line);
        output.push_str("\r\n");
    }
    output.push_str("\r\n");
    
    // Syntax
    if let Some(syntax) = &topic.syntax {
        output.push_str("Syntax:\r\n");
        for line in syntax.lines() {
            output.push_str("  ");
            output.push_str(line);
            output.push_str("\r\n");
        }
        output.push_str("\r\n");
    }
    
    // Examples
    if let Some(examples) = &topic.examples {
        output.push_str("Examples:\r\n");
        for line in examples.lines() {
            output.push_str("  ");
            output.push_str(line);
            output.push_str("\r\n");
        }
        output.push_str("\r\n");
    }
    
    // See also
    if !topic.see_also.is_empty() {
        output.push_str("See also: ");
        output.push_str(&topic.see_also.join(", "));
        output.push_str("\r\n");
    }
    
    output
}

/// Help command - show basic help
pub async fn help_command(
    context: Arc<WorldContext>,
    _entity: EcsEntity,
    _cmd: String,
    _args: Vec<String>,
) -> CommandResult {
    // Get the basic help topic
    let user_role = AccountRole::Player; // TODO: Get actual user role from entity/session
    
    if let Some(topic) = get_help_topic(context, "help", user_role).await {
        CommandResult::Success(format_help_topic(&topic))
    } else {
        // Fallback if help topic not found in database
        CommandResult::Success(
            "\r\nHelp System\r\n\
             \r\n\
             Available help commands:\r\n\
               help              - Show this help\r\n\
               help commands     - List all available commands\r\n\
               help <keyword>    - Get detailed help about a specific topic\r\n\
             \r\n\
             Try 'help commands' to see all available commands.\r\n".to_string()
        )
    }
}

/// Help commands - show list of all commands
pub async fn help_commands_command(
    context: Arc<WorldContext>,
    _entity: EcsEntity,
    _cmd: String,
    _args: Vec<String>,
) -> CommandResult {
    let user_role = AccountRole::Player; // TODO: Get actual user role from entity/session
    
    // Get all command help topics from database
    let db = context.persistence().database();
    let result = sqlx::query_as::<_, (String, String, AccountRole)>(
        "SELECT keyword, title, min_role
         FROM wyldlands.help_topics
         WHERE category = 'Command'
         ORDER BY min_role, keyword"
    )
    .fetch_all(db)
    .await;

    match result {
        Ok(topics) => {
            let mut output = String::from("\r\n╔══════════════════════════════════════════════════════════════╗\r\n");
            output.push_str("║                      Available Commands                      ║\r\n");
            output.push_str("╚══════════════════════════════════════════════════════════════╝\r\n\r\n");
            
            let mut player_commands = Vec::new();
            let mut storyteller_commands = Vec::new();
            let mut builder_commands = Vec::new();
            let mut admin_commands = Vec::new();
            
            for (keyword, title, min_role) in topics {
                // Only show commands the user has permission to use
                if !user_role.has_permission(min_role) {
                    continue;
                }
                
                match min_role {
                    AccountRole::Player => player_commands.push((keyword, title)),
                    AccountRole::Storyteller => storyteller_commands.push((keyword, title)),
                    AccountRole::Builder => builder_commands.push((keyword, title)),
                    AccountRole::Admin => admin_commands.push((keyword, title)),
                }
            }
            
            // Player commands
            if !player_commands.is_empty() {
                output.push_str("Player Commands:\r\n");
                for (keyword, title) in player_commands {
                    output.push_str(&format!("  {:15} - {}\r\n", keyword, title));
                }
                output.push_str("\r\n");
            }
            
            // Storyteller commands
            if !storyteller_commands.is_empty() {
                output.push_str("Storyteller Commands:\r\n");
                for (keyword, title) in storyteller_commands {
                    output.push_str(&format!("  {:15} - {}\r\n", keyword, title));
                }
                output.push_str("\r\n");
            }
            
            // Builder commands
            if !builder_commands.is_empty() {
                output.push_str("Builder Commands:\r\n");
                for (keyword, title) in builder_commands {
                    output.push_str(&format!("  {:15} - {}\r\n", keyword, title));
                }
                output.push_str("\r\n");
            }
            
            // Admin commands
            if !admin_commands.is_empty() {
                output.push_str("Admin Commands:\r\n");
                for (keyword, title) in admin_commands {
                    output.push_str(&format!("  {:15} - {}\r\n", keyword, title));
                }
                output.push_str("\r\n");
            }
            
            output.push_str("Use 'help <command>' for detailed information about a specific command.\r\n");
            
            CommandResult::Success(output)
        }
        Err(e) => {
            tracing::error!("Failed to fetch help topics: {}", e);
            CommandResult::Failure("Failed to load help topics from database".to_string())
        }
    }
}

/// Help keyword - show help for a specific topic
pub async fn help_keyword_command(
    context: Arc<WorldContext>,
    _entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Invalid("Usage: help <keyword>".to_string());
    }
    
    let keyword = args[0].to_lowercase();
    let user_role = AccountRole::Player; // TODO: Get actual user role from entity/session
    
    match get_help_topic(context, &keyword, user_role).await {
        Some(topic) => CommandResult::Success(format_help_topic(&topic)),
        None => CommandResult::Failure(format!(
            "No help available for '{}'. Try 'help commands' to see all available commands.",
            keyword
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_category_enum() {
        // Just verify the enum compiles and can be used
        let _category = HelpCategory::Command;
        let _category2 = HelpCategory::System;
    }

    #[test]
    fn test_help_topic_creation() {
        let topic = HelpTopic {
            keyword: "test".to_string(),
            category: HelpCategory::General,
            title: "Test Topic".to_string(),
            content: "Test content".to_string(),
            syntax: Some("test [args]".to_string()),
            examples: Some("test example".to_string()),
            see_also: vec!["related".to_string()],
            min_role: AccountRole::Player,
        };
        
        assert_eq!(topic.keyword, "test");
        assert_eq!(topic.title, "Test Topic");
    }

    #[test]
    fn test_format_help_topic() {
        let topic = HelpTopic {
            keyword: "test".to_string(),
            category: HelpCategory::Command,
            title: "Test Command".to_string(),
            content: "This is a test command.".to_string(),
            syntax: Some("test <arg>".to_string()),
            examples: Some("test hello".to_string()),
            see_also: vec!["help".to_string()],
            min_role: AccountRole::Player,
        };
        
        let formatted = format_help_topic(&topic);
        assert!(formatted.contains("Test Command"));
        assert!(formatted.contains("This is a test command"));
        assert!(formatted.contains("Syntax:"));
        assert!(formatted.contains("Examples:"));
        assert!(formatted.contains("See also:"));
    }
}


