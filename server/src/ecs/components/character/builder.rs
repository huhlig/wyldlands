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

use crate::ecs::components::character::attributes::{
    AttributeClass, AttributeScores, AttributeType, chargen_attribute_cost,
};
use crate::ecs::components::character::nationality::Nationality;
use crate::ecs::components::character::skills::{
    Skill, Skills, chargen_skill_cost, skill_experience_floor_for_level,
    skill_knowledge_cap_for_level,
};
use crate::ecs::components::character::talents::{Talent, Talents};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Server-side character builder
///
/// This is the authoritative source for character creation state.
/// All validation and game rules are enforced here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterBuilder {
    /// Character name
    pub name: String,

    /// Character age
    pub age: u16,

    /// Character Nationality
    pub nationality: Nationality,

    /// Body Attribute allocations (rank 10-20, starting at 10)
    pub body_attributes: AttributeScores,

    /// Mind Attribute allocations (rank 10-20, starting at 10)
    pub mind_attributes: AttributeScores,

    /// Soul Attribute allocations (rank 10-20, starting at 10)
    pub soul_attributes: AttributeScores,

    /// Selected talents
    pub talents: Talents,

    /// Skill allocations (rank 0-10)
    pub skills: Skills,

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

impl CharacterBuilder {
    /// Create a new character builder with configured point pools
    pub fn new(name: String, max_attribute_talent_points: i32, max_skill_points: i32) -> Self {
        Self {
            name,
            age: 15, // Default starting age
            nationality: Nationality::Aurelian,
            body_attributes: AttributeScores::default(),
            mind_attributes: AttributeScores::default(),
            soul_attributes: AttributeScores::default(),
            talents: Talents::default(),
            skills: Skills::default(),
            attribute_talent_points: max_attribute_talent_points,
            skill_points: max_skill_points,
            max_attribute_talent_points,
            max_skill_points,
            starting_location_id: None,
        }
    }

    /// Set the nationality
    pub fn set_nationality(&mut self, nationality: Nationality) {
        self.nationality = nationality;
    }

    /// Set the starting location
    pub fn set_starting_location(&mut self, location_id: String) {
        self.starting_location_id = Some(location_id);
    }

    /// Increase character age
    pub fn increase_age(&mut self) -> Result<(), String> {
        if self.age + 1 > 60 {
            return Err("Cannot increase age beyond 60".to_string());
        }
        self.age += 1;
        Ok(())
    }

    /// Decrease character age
    pub fn decrease_age(&mut self) -> Result<(), String> {
        if self.age - 1 < 10 {
            return Err("Cannot decrease age below 10".to_string());
        }
        self.age -= 1;
        Ok(())
    }

    /// Get current attribute rank
    pub fn get_attribute(&self, attr: AttributeType) -> i32 {
        match attr {
            AttributeType::BodyOffence => self.body_attributes.score_offence,
            AttributeType::BodyFinesse => self.body_attributes.score_finesse,
            AttributeType::BodyDefence => self.body_attributes.score_defence,
            AttributeType::MindOffence => self.mind_attributes.score_offence,
            AttributeType::MindFinesse => self.mind_attributes.score_finesse,
            AttributeType::MindDefence => self.mind_attributes.score_defence,
            AttributeType::SoulOffence => self.soul_attributes.score_offence,
            AttributeType::SoulFinesse => self.soul_attributes.score_finesse,
            AttributeType::SoulDefence => self.soul_attributes.score_defence,
        }
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
            (current + 1..=new_value).map(chargen_attribute_cost).sum()
        } else {
            // Decreasing: refund costs from new_value+1 to current
            -((new_value + 1..=current)
                .map(chargen_attribute_cost)
                .sum::<i32>())
        };

        if cost_diff > self.attribute_talent_points {
            return Err(format!("Not enough points. Need {} points.", cost_diff));
        }

        match attr {
            AttributeType::BodyOffence => self.body_attributes.score_offence = new_value,
            AttributeType::BodyFinesse => self.body_attributes.score_finesse = new_value,
            AttributeType::BodyDefence => self.body_attributes.score_defence = new_value,
            AttributeType::MindOffence => self.mind_attributes.score_offence = new_value,
            AttributeType::MindFinesse => self.mind_attributes.score_finesse = new_value,
            AttributeType::MindDefence => self.mind_attributes.score_defence = new_value,
            AttributeType::SoulOffence => self.soul_attributes.score_offence = new_value,
            AttributeType::SoulFinesse => self.soul_attributes.score_finesse = new_value,
            AttributeType::SoulDefence => self.soul_attributes.score_defence = new_value,
        }

        match attr.class() {
            AttributeClass::Body => self.body_attributes.update_substats(),
            AttributeClass::Mind => self.mind_attributes.update_substats(),
            AttributeClass::Soul => self.soul_attributes.update_substats(),
        }
        self.attribute_talent_points -= cost_diff;

        Ok(())
    }

    pub fn get_health(&self, class: AttributeClass) -> f32 {
        match class {
            AttributeClass::Body => self.body_attributes.health_current,
            AttributeClass::Mind => self.mind_attributes.health_current,
            AttributeClass::Soul => self.soul_attributes.health_current,
        }
    }

    pub fn get_energy(&self, class: AttributeClass) -> f32 {
        match class {
            AttributeClass::Body => self.body_attributes.energy_current,
            AttributeClass::Mind => self.mind_attributes.energy_current,
            AttributeClass::Soul => self.soul_attributes.energy_current,
        }
    }

    /// Try to add or remove a talent
    pub fn modify_talent(&mut self, talent: Talent, add: bool) -> Result<(), String> {
        if let Some(cost) = talent.cost() {
            if add {
                if self.talents.has_talent(talent) {
                    return Err("Talent already selected".to_string());
                }

                if self.attribute_talent_points < cost {
                    return Err(format!("Not enough points. Need {} points.", cost));
                }

                self.talents.add_talent(talent, 0);
                self.attribute_talent_points -= cost;
            } else {
                if self.talents.has_talent(talent) {
                    self.talents.remove_talent(talent);
                    self.attribute_talent_points += cost;
                } else {
                    return Err("Talent not selected".to_string());
                }
            }
        } else {
            return Err(format!(
                "Talent Not Available at Character Generation: {}",
                talent
            ));
        }
        Ok(())
    }

    /// Get current skill rank
    pub fn get_skill_level(&self, skill: Skill) -> i32 {
        self.skills.level(skill)
    }

    /// Try to modify a skill by delta
    pub fn modify_skill_points(&mut self, skill: Skill, delta: i32) -> Result<(), String> {
        if delta == 0 {
            return Ok(());
        }

        let current = self.get_skill_level(skill);
        let new_level = current + delta;

        if new_level < 0 {
            return Err("Skill cannot go below 0".to_string());
        }
        if new_level > 10 {
            return Err("Skill cannot exceed 10".to_string());
        }

        // Calculate cost difference
        let cost_diff = if delta > 0 {
            // Increasing: sum costs from current+1 to new_value
            (current + 1..=new_level).map(chargen_skill_cost).sum()
        } else {
            // Decreasing: refund costs from new_value+1 to current
            -((new_level + 1..=current)
                .map(chargen_skill_cost)
                .sum::<i32>())
        };

        if cost_diff > self.skill_points {
            return Err(format!(
                "Not enough skill points. Need {} points.",
                cost_diff
            ));
        }

        if new_level == 0 {
            self.skills.remove_skill(skill);
        } else {
            self.skills.add_skill(
                skill,
                skill_experience_floor_for_level(new_level, skill.difficulty()),
                skill_knowledge_cap_for_level(new_level, skill.difficulty()),
            );
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

/// Starting location option from database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(sqlx::FromRow))]
pub struct StartingLocation {
    pub id: String,
    pub name: String,
    pub description: String,
    pub room_id: Uuid,
    pub enabled: bool,
    pub sort_order: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_builder() {
        let builder = CharacterBuilder::new("Test".to_string(), 50, 30);

        assert_eq!(builder.name, "Test");
        assert_eq!(builder.attribute_talent_points, 50);
        assert_eq!(builder.skill_points, 30);
        assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 10);
        assert_eq!(builder.talents.len(), 0);
        assert_eq!(builder.skills.len(), 0);
    }

    #[test]
    fn test_modify_attribute_increase() {
        let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

        // Increase attribute from 10 to 11 (costs 3 points)
        assert!(
            builder
                .modify_attribute(AttributeType::BodyOffence, 1)
                .is_ok()
        );
        assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 11);
        assert_eq!(builder.attribute_talent_points, 47); // 50 - 3
    }

    #[test]
    fn test_modify_attribute_decrease() {
        let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

        // Increase then decrease
        builder
            .modify_attribute(AttributeType::BodyOffence, 1)
            .unwrap();
        assert!(
            builder
                .modify_attribute(AttributeType::BodyOffence, -1)
                .is_ok()
        );
        assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 10);
        assert_eq!(builder.attribute_talent_points, 50); // Refunded
    }

    #[test]
    fn test_modify_attribute_limits() {
        let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

        // Cannot go below 10
        assert!(
            builder
                .modify_attribute(AttributeType::BodyOffence, -1)
                .is_err()
        );

        // Cannot go above 20
        assert!(
            builder
                .modify_attribute(AttributeType::BodyOffence, 11)
                .is_err()
        );
    }

    #[test]
    fn test_modify_talent_add() {
        let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

        // Add talent (costs 5 points)
        assert!(builder.modify_talent(Talent::WeaponMaster, true).is_ok());
        assert_eq!(builder.talents.len(), 1);
        assert_eq!(builder.attribute_talent_points, 45); // 50 - 5
    }

    #[test]
    fn test_modify_talent_remove() {
        let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

        // Add then remove talent
        builder.modify_talent(Talent::WeaponMaster, true).unwrap();
        assert!(builder.modify_talent(Talent::WeaponMaster, false).is_ok());
        assert_eq!(builder.talents.len(), 0);
        assert_eq!(builder.attribute_talent_points, 50); // Refunded
    }

    #[test]
    fn test_modify_talent_duplicate() {
        let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

        builder.modify_talent(Talent::WeaponMaster, true).unwrap();
        // Cannot add same talent twice
        assert!(builder.modify_talent(Talent::WeaponMaster, true).is_err());
    }

    #[test]
    fn test_modify_skill() {
        let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

        // Increase skill from 0 to 1 (costs 1 point)
        assert!(builder.modify_skill_points(Skill::Swordsmanship, 1).is_ok());
        assert_eq!(builder.get_skill_level(Skill::Swordsmanship), 1);
        assert_eq!(builder.skill_points, 29); // 30 - 1
    }

    #[test]
    fn test_modify_skill_remove() {
        let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

        // Add then remove skill
        builder
            .modify_skill_points(Skill::Swordsmanship, 1)
            .unwrap();
        assert!(
            builder
                .modify_skill_points(Skill::Swordsmanship, -1)
                .is_ok()
        );
        assert_eq!(builder.get_skill_level(Skill::Swordsmanship), 0);
        assert!(!builder.skills.has_skill(Skill::Swordsmanship));
    }

    #[test]
    fn test_validate_empty_name() {
        let builder = CharacterBuilder::new("".to_string(), 50, 30);
        assert!(builder.validate().is_err());
        assert!(!builder.is_valid());
    }

    #[test]
    fn test_validate_no_location() {
        let builder = CharacterBuilder::new("Test".to_string(), 50, 30);
        assert!(builder.validate().is_err());
        let errors = builder.validation_errors();
        assert!(errors.iter().any(|e| e.contains("Starting location")));
    }

    #[test]
    fn test_validate_valid() {
        let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);
        builder.set_starting_location("start_location".to_string());
        assert!(builder.validate().is_ok());
        assert!(builder.is_valid());
    }
}
