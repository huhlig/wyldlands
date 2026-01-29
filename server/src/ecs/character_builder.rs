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

//! Server-side character builder
//!
//! This module contains the authoritative character creation logic.
//! All validation and game rules are enforced here.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wyldlands_common::character::{
    AttributeType, Talent, attribute_cost, skill_cost,
};

/// Server-side character builder
///
/// This is the authoritative source for character creation state.
/// All validation and game rules are enforced here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCharacterBuilder {
    /// Character name
    pub name: String,
    
    /// Attribute allocations (rank 10-20, starting at 10)
    pub attributes: HashMap<AttributeType, i32>,
    
    /// Selected talents
    pub talents: Vec<Talent>,
    
    /// Skill allocations (rank 0-10)
    pub skills: HashMap<String, i32>,
    
    /// Available points for attributes and talents (shared pool)
    pub attribute_talent_points: i32,
    
    /// Available points for skills (separate pool)
    pub skill_points: i32,
    
    /// Maximum attribute/talent points (from config)
    pub max_attribute_talent_points: i32,
    
    /// Maximum skill points (from config)
    pub max_skill_points: i32,
    
    /// Selected starting location ID
    pub starting_location_id: Option<String>,
}

impl ServerCharacterBuilder {
    /// Create a new character builder with configured point pools
    pub fn new(
        name: String,
        max_attribute_talent_points: i32,
        max_skill_points: i32,
    ) -> Self {
        let mut attributes = HashMap::new();
        // Initialize all attributes to 10 (base value)
        for attr in AttributeType::all() {
            attributes.insert(attr, 10);
        }
        
        Self {
            name,
            attributes,
            talents: Vec::new(),
            skills: HashMap::new(),
            attribute_talent_points: max_attribute_talent_points,
            skill_points: max_skill_points,
            max_attribute_talent_points,
            max_skill_points,
            starting_location_id: None,
        }
    }
    
    /// Set the starting location
    pub fn set_starting_location(&mut self, location_id: String) {
        self.starting_location_id = Some(location_id);
    }
    
    /// Get current attribute rank
    pub fn get_attribute(&self, attr: AttributeType) -> i32 {
        *self.attributes.get(&attr).unwrap_or(&10)
    }
    
    /// Try to modify an attribute by delta
    pub fn modify_attribute(&mut self, attr: AttributeType, delta: i32) -> Result<(), String> {
        if delta == 0 {
            return Ok(());
        }
        
        let current = self.get_attribute(attr);
        let new_value = current + delta;
        
        if new_value < 10 {
            return Err("Attribute cannot go below 10".to_string());
        }
        if new_value > 20 {
            return Err("Attribute cannot exceed 20".to_string());
        }
        
        // Calculate cost difference
        let cost_diff = if delta > 0 {
            // Increasing: sum costs from current+1 to new_value
            (current + 1..=new_value).map(attribute_cost).sum()
        } else {
            // Decreasing: refund costs from new_value+1 to current
            -((new_value + 1..=current).map(attribute_cost).sum::<i32>())
        };
        
        if cost_diff > self.attribute_talent_points {
            return Err(format!("Not enough points. Need {} points.", cost_diff));
        }
        
        self.attributes.insert(attr, new_value);
        self.attribute_talent_points -= cost_diff;
        Ok(())
    }
    
    /// Try to add or remove a talent
    pub fn modify_talent(&mut self, talent: Talent, add: bool) -> Result<(), String> {
        if add {
            if self.talents.contains(&talent) {
                return Err("Talent already selected".to_string());
            }
            
            let cost = talent.cost();
            if self.attribute_talent_points < cost {
                return Err(format!("Not enough points. Need {} points.", cost));
            }
            
            self.talents.push(talent);
            self.attribute_talent_points -= cost;
        } else {
            if let Some(pos) = self.talents.iter().position(|t| *t == talent) {
                self.talents.remove(pos);
                self.attribute_talent_points += talent.cost();
            } else {
                return Err("Talent not selected".to_string());
            }
        }
        Ok(())
    }
    
    /// Get current skill rank
    pub fn get_skill(&self, skill: &str) -> i32 {
        *self.skills.get(skill).unwrap_or(&0)
    }
    
    /// Try to modify a skill by delta
    pub fn modify_skill(&mut self, skill: String, delta: i32) -> Result<(), String> {
        if delta == 0 {
            return Ok(());
        }
        
        let current = self.get_skill(&skill);
        let new_value = current + delta;
        
        if new_value < 0 {
            return Err("Skill cannot go below 0".to_string());
        }
        if new_value > 10 {
            return Err("Skill cannot exceed 10".to_string());
        }
        
        // Calculate cost difference
        let cost_diff = if delta > 0 {
            // Increasing: sum costs from current+1 to new_value
            (current + 1..=new_value).map(skill_cost).sum()
        } else {
            // Decreasing: refund costs from new_value+1 to current
            -((new_value + 1..=current).map(skill_cost).sum::<i32>())
        };
        
        if cost_diff > self.skill_points {
            return Err(format!("Not enough skill points. Need {} points.", cost_diff));
        }
        
        if new_value == 0 {
            self.skills.remove(&skill);
        } else {
            self.skills.insert(skill, new_value);
        }
        self.skill_points -= cost_diff;
        Ok(())
    }
    
    /// Validate the character for creation
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        if self.name.is_empty() {
            errors.push("Character name is required".to_string());
        }
        
        if self.name.len() > 50 {
            errors.push("Character name too long (max 50 characters)".to_string());
        }
        
        if self.starting_location_id.is_none() {
            errors.push("Starting location must be selected".to_string());
        }
        
        if self.attribute_talent_points < 0 {
            errors.push("Overspent attribute/talent points".to_string());
        }
        
        if self.skill_points < 0 {
            errors.push("Overspent skill points".to_string());
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Check if the character is valid for creation
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
    
    /// Get validation errors
    pub fn validation_errors(&self) -> Vec<String> {
        match self.validate() {
            Ok(()) => Vec::new(),
            Err(errors) => errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_builder() {
        let builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        assert_eq!(builder.name, "Test");
        assert_eq!(builder.attribute_talent_points, 50);
        assert_eq!(builder.skill_points, 30);
        assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 10);
        assert_eq!(builder.talents.len(), 0);
        assert_eq!(builder.skills.len(), 0);
    }

    #[test]
    fn test_modify_attribute_increase() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        // Increase attribute from 10 to 11 (costs 3 points)
        assert!(builder.modify_attribute(AttributeType::BodyOffence, 1).is_ok());
        assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 11);
        assert_eq!(builder.attribute_talent_points, 47); // 50 - 3
    }
    
    #[test]
    fn test_modify_attribute_decrease() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        // Increase then decrease
        builder.modify_attribute(AttributeType::BodyOffence, 1).unwrap();
        assert!(builder.modify_attribute(AttributeType::BodyOffence, -1).is_ok());
        assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 10);
        assert_eq!(builder.attribute_talent_points, 50); // Refunded
    }
    
    #[test]
    fn test_modify_attribute_limits() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        // Cannot go below 10
        assert!(builder.modify_attribute(AttributeType::BodyOffence, -1).is_err());
        
        // Cannot go above 20
        assert!(builder.modify_attribute(AttributeType::BodyOffence, 11).is_err());
    }
    
    #[test]
    fn test_modify_talent_add() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        // Add talent (costs 5 points)
        assert!(builder.modify_talent(Talent::WeaponMaster, true).is_ok());
        assert_eq!(builder.talents.len(), 1);
        assert_eq!(builder.attribute_talent_points, 45); // 50 - 5
    }
    
    #[test]
    fn test_modify_talent_remove() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        // Add then remove talent
        builder.modify_talent(Talent::WeaponMaster, true).unwrap();
        assert!(builder.modify_talent(Talent::WeaponMaster, false).is_ok());
        assert_eq!(builder.talents.len(), 0);
        assert_eq!(builder.attribute_talent_points, 50); // Refunded
    }
    
    #[test]
    fn test_modify_talent_duplicate() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        builder.modify_talent(Talent::WeaponMaster, true).unwrap();
        // Cannot add same talent twice
        assert!(builder.modify_talent(Talent::WeaponMaster, true).is_err());
    }
    
    #[test]
    fn test_modify_skill() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        // Increase skill from 0 to 1 (costs 1 point)
        assert!(builder.modify_skill("Swordsmanship".to_string(), 1).is_ok());
        assert_eq!(builder.get_skill("Swordsmanship"), 1);
        assert_eq!(builder.skill_points, 29); // 30 - 1
    }
    
    #[test]
    fn test_modify_skill_remove() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        // Add then remove skill
        builder.modify_skill("Swordsmanship".to_string(), 1).unwrap();
        assert!(builder.modify_skill("Swordsmanship".to_string(), -1).is_ok());
        assert_eq!(builder.get_skill("Swordsmanship"), 0);
        assert!(!builder.skills.contains_key("Swordsmanship"));
        assert_eq!(builder.skill_points, 30); // Refunded
    }
    
    #[test]
    fn test_validate_empty_name() {
        let builder = ServerCharacterBuilder::new("".to_string(), 50, 30);
        assert!(builder.validate().is_err());
        assert!(!builder.is_valid());
    }
    
    #[test]
    fn test_validate_no_location() {
        let builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        assert!(builder.validate().is_err());
        let errors = builder.validation_errors();
        assert!(errors.iter().any(|e| e.contains("Starting location")));
    }
    
    #[test]
    fn test_validate_valid() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        builder.set_starting_location("start_location".to_string());
        assert!(builder.validate().is_ok());
        assert!(builder.is_valid());
    }
}


