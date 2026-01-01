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

//! Shared character types and utilities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Starting location option from database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct StartingLocation {
    pub id: String,
    pub name: String,
    pub description: String,
    pub room_id: Uuid,
    pub enabled: bool,
    pub sort_order: i32,
}

/// Point costs for attribute ranks (progressive cost)
/// Rank 1-5: 1 point each
/// Rank 6-10: 2 points each
/// Rank 11-15: 3 points each
/// Rank 16-20: 4 points each
pub fn attribute_cost(rank: i32) -> i32 {
    match rank {
        1..=5 => 1,
        6..=10 => 2,
        11..=15 => 3,
        16..=20 => 4,
        _ => 0,
    }
}

/// Calculate total cost for an attribute at a given rank
pub fn total_attribute_cost(rank: i32) -> i32 {
    (1..=rank).map(attribute_cost).sum()
}

/// Point costs for skill ranks (progressive cost)
/// Rank 1-3: 1 point each
/// Rank 4-6: 2 points each
/// Rank 7-9: 3 points each
/// Rank 10: 4 points
pub fn skill_cost(rank: i32) -> i32 {
    match rank {
        1..=3 => 1,
        4..=6 => 2,
        7..=9 => 3,
        10 => 4,
        _ => 0,
    }
}

/// Calculate total cost for a skill at a given rank
pub fn total_skill_cost(rank: i32) -> i32 {
    (1..=rank).map(skill_cost).sum()
}

/// Available talents with fixed costs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Talent {
    // Combat Talents (5 points each)
    WeaponMaster,
    ShieldExpert,
    DualWielder,
    Berserker,
    Tactician,
    
    // Magic Talents (5 points each)
    Spellweaver,
    ElementalAffinity,
    ArcaneScholar,
    Ritualist,
    Channeler,
    AstralProjection,
    
    // Crafting Talents (3 points each)
    MasterCraftsman,
    Artificer,
    Alchemist,
    Enchanter,
    
    // Social Talents (3 points each)
    Diplomat,
    Merchant,
    Leader,
    Performer,
    
    // Survival Talents (3 points each)
    Tracker,
    Forager,
    BeastMaster,
    Survivalist,
    
    // Special Talents (7 points each)
    Prodigy,
    Lucky,
    FastLearner,
    Resilient,
}

impl Talent {
    /// Get the fixed cost of this talent
    pub fn cost(&self) -> i32 {
        match self {
            // Combat talents
            Talent::WeaponMaster | Talent::ShieldExpert | Talent::DualWielder 
            | Talent::Berserker | Talent::Tactician => 5,
            
            // Magic talents
            Talent::Spellweaver | Talent::ElementalAffinity | Talent::ArcaneScholar 
            | Talent::Ritualist | Talent::Channeler | Talent::AstralProjection => 5,
            
            // Crafting talents
            Talent::MasterCraftsman | Talent::Artificer | Talent::Alchemist 
            | Talent::Enchanter => 3,
            
            // Social talents
            Talent::Diplomat | Talent::Merchant | Talent::Leader | Talent::Performer => 3,
            
            // Survival talents
            Talent::Tracker | Talent::Forager | Talent::BeastMaster | Talent::Survivalist => 3,
            
            // Special talents
            Talent::Prodigy | Talent::Lucky | Talent::FastLearner | Talent::Resilient => 7,
        }
    }
    
    /// Get the name of this talent
    pub fn name(&self) -> &'static str {
        match self {
            Talent::WeaponMaster => "Weapon Master",
            Talent::ShieldExpert => "Shield Expert",
            Talent::DualWielder => "Dual Wielder",
            Talent::Berserker => "Berserker",
            Talent::Tactician => "Tactician",
            Talent::Spellweaver => "Spellweaver",
            Talent::ElementalAffinity => "Elemental Affinity",
            Talent::ArcaneScholar => "Arcane Scholar",
            Talent::Ritualist => "Ritualist",
            Talent::Channeler => "Channeler",
            Talent::AstralProjection => "Astral Projection",
            Talent::MasterCraftsman => "Master Craftsman",
            Talent::Artificer => "Artificer",
            Talent::Alchemist => "Alchemist",
            Talent::Enchanter => "Enchanter",
            Talent::Diplomat => "Diplomat",
            Talent::Merchant => "Merchant",
            Talent::Leader => "Leader",
            Talent::Performer => "Performer",
            Talent::Tracker => "Tracker",
            Talent::Forager => "Forager",
            Talent::BeastMaster => "Beast Master",
            Talent::Survivalist => "Survivalist",
            Talent::Prodigy => "Prodigy",
            Talent::Lucky => "Lucky",
            Talent::FastLearner => "Fast Learner",
            Talent::Resilient => "Resilient",
        }
    }
    
    /// Get the description of this talent
    pub fn description(&self) -> &'static str {
        match self {
            Talent::WeaponMaster => "Bonus to all weapon skills",
            Talent::ShieldExpert => "Enhanced shield defense and blocking",
            Talent::DualWielder => "Wield two weapons effectively",
            Talent::Berserker => "Rage in combat for increased damage",
            Talent::Tactician => "Bonus to combat strategy and leadership",
            Talent::Spellweaver => "Cast spells more efficiently",
            Talent::ElementalAffinity => "Enhanced elemental magic",
            Talent::ArcaneScholar => "Bonus to magical knowledge",
            Talent::Ritualist => "Perform powerful rituals",
            Talent::Channeler => "Channel magical energy more effectively",
            Talent::AstralProjection => "Project astral form for extended exploration",
            Talent::MasterCraftsman => "Bonus to all crafting skills",
            Talent::Artificer => "Create magical items",
            Talent::Alchemist => "Enhanced potion and poison creation",
            Talent::Enchanter => "Enchant items with magical properties",
            Talent::Diplomat => "Bonus to persuasion and negotiation",
            Talent::Merchant => "Better prices when trading",
            Talent::Leader => "Inspire and command others",
            Talent::Performer => "Captivate audiences with performance",
            Talent::Tracker => "Enhanced tracking and hunting",
            Talent::Forager => "Find resources more easily",
            Talent::BeastMaster => "Bond with and train animals",
            Talent::Survivalist => "Thrive in harsh environments",
            Talent::Prodigy => "Learn all skills faster",
            Talent::Lucky => "Fortune favors you in all endeavors",
            Talent::FastLearner => "Gain experience more quickly",
            Talent::Resilient => "Recover from damage faster",
        }
    }
    
    /// Get all available talents
    pub fn all() -> Vec<Talent> {
        vec![
            // Combat
            Talent::WeaponMaster, Talent::ShieldExpert, Talent::DualWielder,
            Talent::Berserker, Talent::Tactician,
            // Magic
            Talent::Spellweaver, Talent::ElementalAffinity, Talent::ArcaneScholar,
            Talent::Ritualist, Talent::Channeler, Talent::AstralProjection,
            // Crafting
            Talent::MasterCraftsman, Talent::Artificer, Talent::Alchemist, Talent::Enchanter,
            // Social
            Talent::Diplomat, Talent::Merchant, Talent::Leader, Talent::Performer,
            // Survival
            Talent::Tracker, Talent::Forager, Talent::BeastMaster, Talent::Survivalist,
            // Special
            Talent::Prodigy, Talent::Lucky, Talent::FastLearner, Talent::Resilient,
        ]
    }
}

/// Attribute types
/// Character Attributes
///
/// |           | Body      | Mind    | Soul       |
/// |-----------|-----------|---------|------------|
/// | Power     | Strength  | Acuity  | Authority  |
/// | Control   | Dexterity | Focus   | Resonance  |
/// | Stability | Fortitude | Resolve | Permanence |
/// | Health    | Vitality  | Sanity  | Stability  |
/// | Energy    | Stamina   | Psyche  | Aether     |
/// | Regen     | Recovery  | Clarity | Dominion   |
///
/// * Power - How strongly this aspect can act upon the world.
/// * Control - How precisely and efficiently power is applied.
/// * Stability - How well this aspect resists disruption.
/// * Health - How resilient this aspect is to damage and stress.
/// * Energy - What is spent to perform actions.
/// * Regen - How quickly the aspect restores itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeType {
    // Body Attributes
    BodyOffence,
    BodyFinesse,
    BodyDefence,
    
    // Mind Attributes
    MindOffence,
    MindFinesse,
    MindDefence,
    
    // Soul Attributes
    SoulOffence,
    SoulFinesse,
    SoulDefence,
}

impl AttributeType {
    pub fn name(&self) -> &'static str {
        match self {
            AttributeType::BodyOffence => "Body Offence",
            AttributeType::BodyFinesse => "Body Finesse",
            AttributeType::BodyDefence => "Body Defence",
            AttributeType::MindOffence => "Mind Offence",
            AttributeType::MindFinesse => "Mind Finesse",
            AttributeType::MindDefence => "Mind Defence",
            AttributeType::SoulOffence => "Soul Offence",
            AttributeType::SoulFinesse => "Soul Finesse",
            AttributeType::SoulDefence => "Soul Defence",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            AttributeType::BodyOffence => "Physical attack power and damage",
            AttributeType::BodyFinesse => "Physical accuracy and critical hits",
            AttributeType::BodyDefence => "Physical damage resistance",
            AttributeType::MindOffence => "Mental attack power",
            AttributeType::MindFinesse => "Mental accuracy and focus",
            AttributeType::MindDefence => "Mental damage resistance",
            AttributeType::SoulOffence => "Spiritual attack power",
            AttributeType::SoulFinesse => "Spiritual accuracy and connection",
            AttributeType::SoulDefence => "Spiritual damage resistance",
        }
    }
    
    pub fn all() -> Vec<AttributeType> {
        vec![
            AttributeType::BodyOffence, AttributeType::BodyFinesse, AttributeType::BodyDefence,
            AttributeType::MindOffence, AttributeType::MindFinesse, AttributeType::MindDefence,
            AttributeType::SoulOffence, AttributeType::SoulFinesse, AttributeType::SoulDefence,
        ]
    }
}


/// Character builder state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterBuilder {
    /// Character name
    pub name: String,
    
    /// Attribute allocations (rank 0-20)
    pub attributes: HashMap<AttributeType, i32>,
    
    /// Selected talents
    pub talents: Vec<Talent>,
    
    /// Skill allocations (rank 0-10)
    pub skills: HashMap<String, i32>,
    
    /// Available points for attributes and talents (shared pool)
    pub attribute_talent_points: i32,
    
    /// Available points for skills (separate pool)
    pub skill_points: i32,
    
    /// Maximum attribute/talent points
    pub max_attribute_talent_points: i32,
    
    /// Maximum skill points
    pub max_skill_points: i32,
    
    /// Selected starting location ID (optional until chosen)
    pub starting_location_id: Option<String>,
}

impl CharacterBuilder {
    /// Create a new character builder with default point pools
    pub fn new(name: String) -> Self {
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
            attribute_talent_points: 50, // Starting points for attributes/talents
            skill_points: 30, // Starting points for skills
            max_attribute_talent_points: 50,
            max_skill_points: 30,
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
    
    /// Try to increase an attribute by 1 rank
    pub fn increase_attribute(&mut self, attr: AttributeType) -> Result<(), String> {
        let current = self.get_attribute(attr);
        if current >= 20 {
            return Err("Attribute is already at maximum (20)".to_string());
        }
        
        let cost = attribute_cost(current + 1);
        if self.attribute_talent_points < cost {
            return Err(format!("Not enough points. Need {} points.", cost));
        }
        
        self.attributes.insert(attr, current + 1);
        self.attribute_talent_points -= cost;
        Ok(())
    }
    
    /// Try to decrease an attribute by 1 rank
    pub fn decrease_attribute(&mut self, attr: AttributeType) -> Result<(), String> {
        let current = self.get_attribute(attr);
        if current <= 10 {
            return Err("Attribute is already at minimum (10)".to_string());
        }
        
        let refund = attribute_cost(current);
        self.attributes.insert(attr, current - 1);
        self.attribute_talent_points += refund;
        Ok(())
    }
    
    /// Try to add a talent
    pub fn add_talent(&mut self, talent: Talent) -> Result<(), String> {
        if self.talents.contains(&talent) {
            return Err("Talent already selected".to_string());
        }
        
        let cost = talent.cost();
        if self.attribute_talent_points < cost {
            return Err(format!("Not enough points. Need {} points.", cost));
        }
        
        self.talents.push(talent);
        self.attribute_talent_points -= cost;
        Ok(())
    }
    
    /// Try to remove a talent
    pub fn remove_talent(&mut self, talent: Talent) -> Result<(), String> {
        if let Some(pos) = self.talents.iter().position(|t| *t == talent) {
            self.talents.remove(pos);
            self.attribute_talent_points += talent.cost();
            Ok(())
        } else {
            Err("Talent not selected".to_string())
        }
    }
    
    /// Get current skill rank
    pub fn get_skill(&self, skill: &str) -> i32 {
        *self.skills.get(skill).unwrap_or(&0)
    }
    
    /// Try to increase a skill by 1 rank
    pub fn increase_skill(&mut self, skill: String) -> Result<(), String> {
        let current = self.get_skill(&skill);
        if current >= 10 {
            return Err("Skill is already at maximum (10)".to_string());
        }
        
        let cost = skill_cost(current + 1);
        if self.skill_points < cost {
            return Err(format!("Not enough skill points. Need {} points.", cost));
        }
        
        self.skills.insert(skill, current + 1);
        self.skill_points -= cost;
        Ok(())
    }
    
    /// Try to decrease a skill by 1 rank
    pub fn decrease_skill(&mut self, skill: &str) -> Result<(), String> {
        let current = self.get_skill(skill);
        if current <= 0 {
            return Err("Skill is already at minimum (0)".to_string());
        }
        
        let refund = skill_cost(current);
        if current == 1 {
            self.skills.remove(skill);
        } else {
            self.skills.insert(skill.to_string(), current - 1);
        }
        self.skill_points += refund;
        Ok(())
    }
    
    /// Calculate total points spent on attributes
    pub fn total_attribute_points_spent(&self) -> i32 {
        self.attributes.iter()
            .map(|(_, &rank)| total_attribute_cost(rank.saturating_sub(10)))
            .sum()
    }
    
    /// Calculate total points spent on talents
    pub fn total_talent_points_spent(&self) -> i32 {
        self.talents.iter().map(|t| t.cost()).sum()
    }
    
    /// Calculate total points spent on skills
    pub fn total_skill_points_spent(&self) -> i32 {
        self.skills.iter()
            .map(|(_, &rank)| total_skill_cost(rank))
            .sum()
    }
    
    /// Check if the character is valid for creation
    pub fn is_valid(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Character name is required".to_string());
        }
        
        if self.starting_location_id.is_none() {
            return Err("Starting location must be selected".to_string());
        }
        
        // Can have unspent points, but not negative
        if self.attribute_talent_points < 0 {
            return Err("Overspent attribute/talent points".to_string());
        }
        
        if self.skill_points < 0 {
            return Err("Overspent skill points".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_attribute_costs() {
        assert_eq!(attribute_cost(1), 1);
        assert_eq!(attribute_cost(5), 1);
        assert_eq!(attribute_cost(6), 2);
        assert_eq!(attribute_cost(10), 2);
        assert_eq!(attribute_cost(11), 3);
        assert_eq!(attribute_cost(20), 4);
        
        assert_eq!(total_attribute_cost(5), 5);
        assert_eq!(total_attribute_cost(10), 15);
    }
    
    #[test]
    fn test_skill_costs() {
        assert_eq!(skill_cost(1), 1);
        assert_eq!(skill_cost(3), 1);
        assert_eq!(skill_cost(4), 2);
        assert_eq!(skill_cost(10), 4);

        assert_eq!(total_skill_cost(3), 3);
        assert_eq!(total_skill_cost(6), 9);
        assert_eq!(total_skill_cost(10), 22);  // 1+1+1+2+2+2+3+3+3+4 = 22
    }
    
    #[test]
    fn test_character_builder() {
        let mut builder = CharacterBuilder::new("Test".to_string());

        // Test attribute increase
        assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 10);
        builder.increase_attribute(AttributeType::BodyOffence).unwrap();
        assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 11);
        assert_eq!(builder.attribute_talent_points, 47);  // 50 - 3 (cost of rank 11)


        // Test talent
        builder.add_talent(Talent::WeaponMaster).unwrap();
        assert_eq!(builder.attribute_talent_points, 42);  // 47 - 5 (WeaponMaster cost)
        
        // Test skill
        builder.increase_skill("Swordsmanship".to_string()).unwrap();
        assert_eq!(builder.get_skill("Swordsmanship"), 1);
        assert_eq!(builder.skill_points, 29);
    }
}

