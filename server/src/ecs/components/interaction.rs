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

//! Interaction components for commands and interactivity

use serde::{Deserialize, Serialize};

/// Marks entities that can receive and execute commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commandable {
    pub command_queue: Vec<QueuedCommand>,
    pub max_queue_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub priority: u8,
}

impl Commandable {
    /// Create a new commandable entity
    pub fn new() -> Self {
        Self {
            command_queue: Vec::new(),
            max_queue_size: 10,
        }
    }
    
    /// Queue a command for execution
    pub fn queue_command(&mut self, command: String, args: Vec<String>) -> bool {
        if self.command_queue.len() >= self.max_queue_size {
            return false;
        }
        self.command_queue.push(QueuedCommand {
            command,
            args,
            priority: 0,
        });
        true
    }
    
    /// Get the next command from the queue
    pub fn next_command(&mut self) -> Option<QueuedCommand> {
        if self.command_queue.is_empty() {
            None
        } else {
            Some(self.command_queue.remove(0))
        }
    }
}

impl Default for Commandable {
    fn default() -> Self {
        Self::new()
    }
}

/// Marks entities that can be interacted with
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interactable {
    pub interactions: Vec<Interaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub verb: String,
    pub description: String,
    pub requires_item: Option<String>,
}

impl Interactable {
    /// Create a new interactable entity
    pub fn new() -> Self {
        Self {
            interactions: Vec::new(),
        }
    }
    
    /// Add an interaction
    pub fn add_interaction(&mut self, verb: String, description: String) {
        self.interactions.push(Interaction {
            verb,
            description,
            requires_item: None,
        });
    }
    
    /// Get all available interactions
    pub fn get_interactions(&self) -> &[Interaction] {
        &self.interactions
    }
}

impl Default for Interactable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_commandable_queue() {
        let mut cmd = Commandable::new();
        assert!(cmd.queue_command("move".into(), vec!["north".into()]));
        assert!(cmd.next_command().is_some());
        assert!(cmd.next_command().is_none());
    }
    
    #[test]
    fn test_commandable_max_queue() {
        let mut cmd = Commandable::new();
        cmd.max_queue_size = 2;
        
        assert!(cmd.queue_command("cmd1".into(), vec![]));
        assert!(cmd.queue_command("cmd2".into(), vec![]));
        assert!(!cmd.queue_command("cmd3".into(), vec![]));
    }
    
    #[test]
    fn test_interactable() {
        let mut inter = Interactable::new();
        inter.add_interaction("pull".into(), "Pull the lever".into());
        
        assert_eq!(inter.get_interactions().len(), 1);
        assert_eq!(inter.get_interactions()[0].verb, "pull");
    }
}


