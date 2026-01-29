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

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Individual skill
/// Maps to: entity_skills table (one row per skill)
///
/// ## Skill Levels
/// * **No Skill – Untrained**
///   _You fumble through the motions, lacking even the basics. Every attempt is clumsy, uncertain, and prone to failure._
/// * ***Level 1 – Apprentice**
///   _You have taken your first steps under guidance. You can mimic techniques but rely heavily on instruction to succeed._
/// * ***Level 2 – Novice**
///   _You grasp the fundamentals and can act without constant supervision, though your movements still carry hesitation and error._
/// * ***Level 3 – Initiate**
///   _The principles have become familiar. You begin to understand the “why” behind the motions, laying the foundation for true growth._
/// * **Level 4 – Adept**
///   _Skill now flows with confidence. You handle common challenges with competence and surprise others with flashes of talent._
/// * **Level 5 – Journeyman**
///   _Seasoned through practice and repetition, you perform reliably in all but the most trying circumstances. Your craft is trusted._
/// * **Level 6 – Master**
///   _Your skill is unmistakable. With precision and control, you shape outcomes rather than merely respond to them. Others look to you for teaching._
/// * **Level 7 – Expert**
///   _Your mastery is honed to brilliance. You see patterns invisible to most, executing techniques with effortless excellence._
/// * **Level 8 – Paragon**
///   _You are the living embodiment of your art. Your every action demonstrates flawless form, setting the standard by which all others are judged._
/// * **Level 9 – Mythical**
///   _Stories struggle to capture your feats. Mortals whisper your name in awe, for your abilities defy reason and seem touched by the divine._
/// * **Level 10 – Legendary**
///   _Your skill has transcended the bounds of time and mortality. You are not merely remembered—you are enshrined in myth, an eternal icon of perfection._
///
/// ### **Experience Gain Formula**
///
/// ΔE = B * (1 + K/M)
///
/// Where:
/// - **ΔE** = Experience gained per action
/// - **B** = Base experience gain (determined by difficulty of the action)
/// - **K** = Current Knowledge level of the skill
/// - **M** = Maximum possible Knowledge level for that skill (acts as a normalization factor)
///
/// ### **How It Works**
/// - At **K = 0**, the character still gains Experience but at the slowest possible rate (**just the base amount**).
/// - As **Knowledge increases**, Experience gain speeds up.
/// - When **\(K = M\)**, Experience is gained at **double the base rate**.
/// - This creates a system where **higher Knowledge accelerates learning by up to 2x**, but even without Knowledge, Experience can still increase—just very slowly.
///
/// ### **Equations**
/// L = Level `0-10`
/// L = min(floor(sqrt(XP/Diff)), 10)
///
/// XP(L) = Maximum Experience for that level
/// XP(L) = M = A * L^2
///
/// ΔE = Experience gained per action
/// ΔE = B * (1 + K/M)
///
/// A = Skill Difficulty Experience Gain Rate
/// B = XP Difficulty Rate
/// K = Current Knowledge
/// M = Knowledge Cap for current level
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Skill {
    pub experience: i32,
    pub knowledge: i32,
}

/// Character skills collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skills {
    pub skills: HashMap<SkillId, Skill>,
}

impl Skills {
    /// Create a new empty skill set
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Get a skill by ID
    pub fn get(&self, id: SkillId) -> Option<&Skill> {
        self.skills.get(&id)
    }

    /// Set a skill
    pub fn set(&mut self, id: SkillId, skill: Skill) {
        self.skills.insert(id, skill);
    }

    /// Get the Level of a Skill by ID (0-10)
    ///
    /// L = min(floor(sqrt(XP/D)), 10)
    /// D = Difficulty Scalar
    pub fn level(&self, id: SkillId) -> i32 {
        if let Some(skill) = self.skills.get(&id) {
            if let Some(skill_def) = SkillRegistry::get_skill_by_id(id) {
                level_from_experience(skill.experience, skill_def.difficulty)
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Get Knowledge Cap for Current Level
    ///
    /// XP(L) = Maximum Experience for that level
    /// XP(L) = M = A * L^2
    pub fn knowledge_cap(&self, id: SkillId) -> i32 {
        if let Some(skill_def) = SkillRegistry::get_skill_by_id(id) {
            knowledge_cap(self.level(id), skill_def.difficulty)
        } else {
            0
        }
    }

    /// Improve a skill by adding base experience points
    /// ΔE = B * (1 + K/M)
    /// P = Action Points
    /// K = Current Knowledge
    /// M = Knowledge Cap for current level
    pub fn improve(&mut self, id: SkillId, points: i32) {
        let current_level = self.level(id);
        if let Some(skill) = self.skills.get_mut(&id) {
            if let Some(skill_def) = SkillRegistry::get_skill_by_id(id) {
                let knowledge_cap = knowledge_cap(current_level, skill_def.difficulty);
                if knowledge_cap == 0 {
                    skill.experience += points;
                } else {
                    skill.experience += points * (1 + skill.knowledge / knowledge_cap);
                }
            }
        }
    }

    /// Train a skill by adding knowledge points
    pub fn train(&mut self, id: SkillId, points: i32) {
        let next_level = 1 + self.level(id);
        if let Some(skill) = self.skills.get_mut(&id) {
            if let Some(skill_def) = SkillRegistry::get_skill_by_id(id) {
                // Calculate knowledge cap for next level.
                let knowledge_cap = knowledge_cap(next_level, skill_def.difficulty);
                let knowledge = skill.knowledge + (points * (1 + skill.knowledge / knowledge_cap));
                skill.knowledge = i32::min(knowledge, knowledge_cap);
            }
        }
    }
}

impl Default for Skills {
    fn default() -> Self {
        Self::new()
    }
}

/// Skill difficulty levels
/// Determines base experience gain and maximum knowledge cap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillDifficulty {
    /// Very Easy - Quick to learn, low skill ceiling
    /// XP Rate: 2.0, Knowledge Rate: 2.0
    VeryEasy,
    /// Easy - Straightforward to learn
    /// XP Rate: 1.5, Knowledge Rate: 1.5
    Easy,
    /// Moderate - Standard difficulty
    /// XP Rate: 1.0, Knowledge Rate: 1.0
    Moderate,
    /// Hard - Challenging to master
    /// XP Rate: 0.75, Knowledge Rate: 0.75
    Hard,
    /// Very Hard - Extremely difficult to master
    /// XP Rate: 0.5, Knowledge Rate: 0.5
    VeryHard,
    /// Legendary - Near impossible to fully master
    /// XP Rate: 0.25, Knowledge Rate: 0.25
    Legendary,
}

impl SkillDifficulty {
    /// Skill Difficulty Scalar
    /// Higher Numbers are more challenging.
    pub fn difficulty(&self) -> i32 {
        match self {
            SkillDifficulty::VeryEasy => 1,
            SkillDifficulty::Easy => 2,
            SkillDifficulty::Moderate => 4,
            SkillDifficulty::Hard => 8,
            SkillDifficulty::VeryHard => 16,
            SkillDifficulty::Legendary => 32,
        }
    }
}

/// Skill categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillCategory {
    Combat,
    Magic,
    Crafting,
    Gathering,
    Social,
    Survival,
    Knowledge,
    Psionic,
    Akashic,
}

/// Unique skill identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillId {
    // Combat Skills
    Swordsmanship,
    Axemanship,
    Spearmanship,
    Archery,
    Crossbows,
    Daggers,
    Unarmed,
    Shields,
    Parrying,
    Dodging,

    // Magic Skills
    Evocation,
    Conjuration,
    Illusion,
    Enchantment,
    Divination,
    Necromancy,
    Transmutation,
    Abjuration,

    // Crafting Skills
    Blacksmithing,
    Armorsmithing,
    Weaponsmithing,
    Leatherworking,
    Tailoring,
    Jewelcrafting,
    Alchemy,
    Cooking,
    Carpentry,
    Masonry,

    // Gathering Skills
    Mining,
    Herbalism,
    Skinning,
    Fishing,
    Logging,
    Foraging,

    // Social Skills
    Persuasion,
    Intimidation,
    Deception,
    Insight,
    Performance,
    Leadership,
    Bartering,

    // Survival Skills
    Tracking,
    Stealth,
    Lockpicking,
    Trapping,
    FirstAid,
    Navigation,
    AnimalHandling,

    // Knowledge Skills
    History,
    Arcana,
    Nature,
    Religion,
    Medicine,
    Engineering,
}

impl SkillId {
    /// Get the skill's display name
    pub fn name(&self) -> &'static str {
        match self {
            // Combat
            SkillId::Swordsmanship => "Swordsmanship",
            SkillId::Axemanship => "Axemanship",
            SkillId::Spearmanship => "Spearmanship",
            SkillId::Archery => "Archery",
            SkillId::Crossbows => "Crossbows",
            SkillId::Daggers => "Daggers",
            SkillId::Unarmed => "Unarmed Combat",
            SkillId::Shields => "Shields",
            SkillId::Parrying => "Parrying",
            SkillId::Dodging => "Dodging",

            // Magic
            SkillId::Evocation => "Evocation",
            SkillId::Conjuration => "Conjuration",
            SkillId::Illusion => "Illusion",
            SkillId::Enchantment => "Enchantment",
            SkillId::Divination => "Divination",
            SkillId::Necromancy => "Necromancy",
            SkillId::Transmutation => "Transmutation",
            SkillId::Abjuration => "Abjuration",

            // Crafting
            SkillId::Blacksmithing => "Blacksmithing",
            SkillId::Armorsmithing => "Armorsmithing",
            SkillId::Weaponsmithing => "Weaponsmithing",
            SkillId::Leatherworking => "Leatherworking",
            SkillId::Tailoring => "Tailoring",
            SkillId::Jewelcrafting => "Jewelcrafting",
            SkillId::Alchemy => "Alchemy",
            SkillId::Cooking => "Cooking",
            SkillId::Carpentry => "Carpentry",
            SkillId::Masonry => "Masonry",

            // Gathering
            SkillId::Mining => "Mining",
            SkillId::Herbalism => "Herbalism",
            SkillId::Skinning => "Skinning",
            SkillId::Fishing => "Fishing",
            SkillId::Logging => "Logging",
            SkillId::Foraging => "Foraging",

            // Social
            SkillId::Persuasion => "Persuasion",
            SkillId::Intimidation => "Intimidation",
            SkillId::Deception => "Deception",
            SkillId::Insight => "Insight",
            SkillId::Performance => "Performance",
            SkillId::Leadership => "Leadership",
            SkillId::Bartering => "Bartering",

            // Survival
            SkillId::Tracking => "Tracking",
            SkillId::Stealth => "Stealth",
            SkillId::Lockpicking => "Lockpicking",
            SkillId::Trapping => "Trapping",
            SkillId::FirstAid => "First Aid",
            SkillId::Navigation => "Navigation",
            SkillId::AnimalHandling => "Animal Handling",

            // Knowledge
            SkillId::History => "History",
            SkillId::Arcana => "Arcana",
            SkillId::Nature => "Nature",
            SkillId::Religion => "Religion",
            SkillId::Medicine => "Medicine",
            SkillId::Engineering => "Engineering",
        }
    }

    /// Get all skill IDs
    pub fn all() -> Vec<SkillId> {
        vec![
            // Combat
            SkillId::Swordsmanship,
            SkillId::Axemanship,
            SkillId::Spearmanship,
            SkillId::Archery,
            SkillId::Crossbows,
            SkillId::Daggers,
            SkillId::Unarmed,
            SkillId::Shields,
            SkillId::Parrying,
            SkillId::Dodging,
            // Magic
            SkillId::Evocation,
            SkillId::Conjuration,
            SkillId::Illusion,
            SkillId::Enchantment,
            SkillId::Divination,
            SkillId::Necromancy,
            SkillId::Transmutation,
            SkillId::Abjuration,
            // Crafting
            SkillId::Blacksmithing,
            SkillId::Armorsmithing,
            SkillId::Weaponsmithing,
            SkillId::Leatherworking,
            SkillId::Tailoring,
            SkillId::Jewelcrafting,
            SkillId::Alchemy,
            SkillId::Cooking,
            SkillId::Carpentry,
            SkillId::Masonry,
            // Gathering
            SkillId::Mining,
            SkillId::Herbalism,
            SkillId::Skinning,
            SkillId::Fishing,
            SkillId::Logging,
            SkillId::Foraging,
            // Social
            SkillId::Persuasion,
            SkillId::Intimidation,
            SkillId::Deception,
            SkillId::Insight,
            SkillId::Performance,
            SkillId::Leadership,
            SkillId::Bartering,
            // Survival
            SkillId::Tracking,
            SkillId::Stealth,
            SkillId::Lockpicking,
            SkillId::Trapping,
            SkillId::FirstAid,
            SkillId::Navigation,
            SkillId::AnimalHandling,
            // Knowledge
            SkillId::History,
            SkillId::Arcana,
            SkillId::Nature,
            SkillId::Religion,
            SkillId::Medicine,
            SkillId::Engineering,
        ]
    }

    /// Convert SkillId to string for database storage
    pub fn to_string(&self) -> String {
        self.name().to_string()
    }

    /// Parse SkillId from string (for database loading)
    pub fn from_string(s: &str) -> Option<SkillId> {
        Self::all().into_iter().find(|id| id.name() == s)
    }
}

impl fmt::Display for SkillId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for SkillId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s).ok_or_else(|| format!("Unknown skill: {}", s))
    }
}

/// Complete skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: SkillId,
    pub name: &'static str,
    pub description: &'static str,
    pub category: SkillCategory,
    pub difficulty: SkillDifficulty,
}

#[derive(Debug, Default)]
pub struct SkillRegistry {
    skills: HashMap<SkillId, SkillDefinition>,
    names: HashMap<String, SkillId>,
}

impl SkillRegistry {
    /// Get a skill definition by ID
    pub fn get_skill_by_id(id: SkillId) -> Option<&'static SkillDefinition> {
        SKILL_REGISTRY.skills.get(&id)
    }

    /// Get a skill definition by name
    pub fn get_skill_by_name(name: &str) -> Option<&'static SkillDefinition> {
        SKILL_REGISTRY.skills.get(
            &SKILL_REGISTRY
                .names
                .get(name.to_ascii_lowercase().as_str())
                .copied()?,
        )
    }

    /// Get all skills in a category
    pub fn get_skills_by_category(category: SkillCategory) -> Vec<&'static SkillDefinition> {
        SKILL_REGISTRY
            .skills
            .values()
            .filter(|skill| skill.category == category)
            .collect()
    }

    /// Insert Skills consistently
    fn insert(&mut self, skill_definition: SkillDefinition) {
        self.names.insert(
            skill_definition.name.to_ascii_lowercase(),
            skill_definition.id,
        );
        self.skills.insert(skill_definition.id, skill_definition);
    }
}

/// Global skill registry
pub static SKILL_REGISTRY: Lazy<SkillRegistry> = Lazy::new(|| {
    let mut registry = SkillRegistry::default();

    // Combat Skills
    registry.insert(SkillDefinition {
        id: SkillId::Swordsmanship,
        name: "Swordsmanship",
        description: "The art of wielding swords in combat",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Axemanship,
        name: "Axemanship",
        description: "Proficiency with axes and similar chopping weapons",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Spearmanship,
        name: "Spearmanship",
        description: "Skill with spears, polearms, and reach weapons",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Archery,
        name: "Archery",
        description: "Precision and power with bows",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Crossbows,
        name: "Crossbows",
        description: "Operating and maintaining crossbows",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Easy,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Daggers,
        name: "Daggers",
        description: "Quick strikes with small blades",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Easy,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Unarmed,
        name: "Unarmed Combat",
        description: "Fighting without weapons using fists, kicks, and grappling",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Shields,
        name: "Shields",
        description: "Defensive techniques with shields",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Parrying,
        name: "Parrying",
        description: "Deflecting attacks with weapons",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Dodging,
        name: "Dodging",
        description: "Avoiding attacks through agility",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Moderate,
    });

    // Magic Skills
    registry.insert(SkillDefinition {
        id: SkillId::Evocation,
        name: "Evocation",
        description: "Channeling raw magical energy into destructive force",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Conjuration,
        name: "Conjuration",
        description: "Summoning creatures and objects from other planes",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::VeryHard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Illusion,
        name: "Illusion",
        description: "Creating false sensory experiences",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Enchantment,
        name: "Enchantment",
        description: "Influencing minds and imbuing objects with magic",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::VeryHard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Divination,
        name: "Divination",
        description: "Perceiving hidden truths and future events",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::Legendary,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Necromancy,
        name: "Necromancy",
        description: "Manipulating life force and commanding the undead",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::VeryHard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Transmutation,
        name: "Transmutation",
        description: "Altering the physical properties of matter",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Abjuration,
        name: "Abjuration",
        description: "Creating protective barriers and dispelling magic",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::Hard,
    });

    // Crafting Skills
    registry.insert(SkillDefinition {
        id: SkillId::Blacksmithing,
        name: "Blacksmithing",
        description: "Forging metal items at the anvil",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Armorsmithing,
        name: "Armorsmithing",
        description: "Crafting protective armor",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Weaponsmithing,
        name: "Weaponsmithing",
        description: "Forging weapons of war",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::VeryHard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Leatherworking,
        name: "Leatherworking",
        description: "Working with leather and hides",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Tailoring,
        name: "Tailoring",
        description: "Sewing cloth garments and items",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Easy,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Jewelcrafting,
        name: "Jewelcrafting",
        description: "Creating jewelry and cutting gems",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Alchemy,
        name: "Alchemy",
        description: "Brewing potions and elixirs",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Cooking,
        name: "Cooking",
        description: "Preparing food and beverages",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Easy,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Carpentry,
        name: "Carpentry",
        description: "Working with wood to create structures and items",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Masonry,
        name: "Masonry",
        description: "Building with stone and brick",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Hard,
    });

    // Gathering Skills
    registry.insert(SkillDefinition {
        id: SkillId::Mining,
        name: "Mining",
        description: "Extracting ore and minerals from the earth",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Herbalism,
        name: "Herbalism",
        description: "Gathering and identifying plants",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Skinning,
        name: "Skinning",
        description: "Harvesting hides and pelts from creatures",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::Easy,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Fishing,
        name: "Fishing",
        description: "Catching fish from water",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::Easy,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Logging,
        name: "Logging",
        description: "Felling trees and harvesting wood",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::Easy,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Foraging,
        name: "Foraging",
        description: "Finding food and useful items in the wild",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::VeryEasy,
    });

    // Social Skills
    registry.insert(SkillDefinition {
        id: SkillId::Persuasion,
        name: "Persuasion",
        description: "Convincing others through charm and reason",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Intimidation,
        name: "Intimidation",
        description: "Influencing others through threats and fear",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Deception,
        name: "Deception",
        description: "Lying and misleading others convincingly",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Insight,
        name: "Insight",
        description: "Reading people's intentions and emotions",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Performance,
        name: "Performance",
        description: "Entertaining others through art and showmanship",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Leadership,
        name: "Leadership",
        description: "Inspiring and commanding others",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::VeryHard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Bartering,
        name: "Bartering",
        description: "Negotiating favorable trades and prices",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Moderate,
    });

    // Survival Skills
    registry.insert(SkillDefinition {
        id: SkillId::Tracking,
        name: "Tracking",
        description: "Following trails and reading signs",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Stealth,
        name: "Stealth",
        description: "Moving silently and remaining hidden",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Lockpicking,
        name: "Lockpicking",
        description: "Opening locks without keys",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Trapping,
        name: "Trapping",
        description: "Setting and disarming traps",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::FirstAid,
        name: "First Aid",
        description: "Treating wounds and ailments in the field",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Navigation,
        name: "Navigation",
        description: "Finding your way using landmarks and tools",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::AnimalHandling,
        name: "Animal Handling",
        description: "Training and controlling animals",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Hard,
    });

    // Knowledge Skills
    registry.insert(SkillDefinition {
        id: SkillId::History,
        name: "History",
        description: "Knowledge of past events and civilizations",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Arcana,
        name: "Arcana",
        description: "Understanding of magical theory and practice",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::Hard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Nature,
        name: "Nature",
        description: "Knowledge of flora, fauna, and natural phenomena",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Religion,
        name: "Religion",
        description: "Understanding of deities, faiths, and rituals",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::Moderate,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Medicine,
        name: "Medicine",
        description: "Advanced healing and anatomical knowledge",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::VeryHard,
    });

    registry.insert(SkillDefinition {
        id: SkillId::Engineering,
        name: "Engineering",
        description: "Understanding of mechanisms and construction",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::Hard,
    });

    registry
});

/// ### **Equations**
///
/// XP(L) = Maximum Experience for that level
/// XP(L) = M = A * L^2
///
/// ΔE = Experience gained per action
/// ΔE = B * (1 + K/M)
///
/// A = Skill Difficulty Experience Gain Rate
/// B = XP Difficulty Rate
/// K = Current Knowledge
/// M = Knowledge Cap for current level

/// Get Level from Experience (0-10)
///
/// ## Equation
/// L = min(floor(sqrt(XP/Diff)), 10)
fn level_from_experience(exp: i32, difficulty: SkillDifficulty) -> i32 {
    i32::min(
        f32::floor(f32::sqrt(exp as f32 / difficulty.difficulty() as f32)) as i32,
        10,
    )
}

/// Knowledge Cap for level
///
/// ## Equation
/// XP(L) = M = A * L^2
fn knowledge_cap(level: i32, difficulty: SkillDifficulty) -> i32 {
    difficulty.difficulty() * level.pow(2)
}

/// Experience gained per action
///
/// ## Equation
/// ΔE = B * (1 + K/M)
fn experience_gain(points: i32, knowledge: i32, difficulty: SkillDifficulty) -> i32 {
    let cap = knowledge_cap(level_from_experience(points, difficulty), difficulty);
    if cap == 0 {
        points
    } else {
        points * (1 + knowledge / cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_registry() {
        let skill = SkillRegistry::get_skill_by_id(SkillId::Swordsmanship).unwrap();
        assert_eq!(skill.name, "Swordsmanship");
        assert_eq!(skill.category, SkillCategory::Combat);
    }

    #[test]
    fn test_experience_calculation() {
        let skill = Skill {
            experience: 0,
            knowledge: 0,
        };
        let skill_def = SkillRegistry::get_skill_by_id(SkillId::Swordsmanship).unwrap();
        let xp_at_0 = experience_gain(0, skill.knowledge, skill_def.difficulty);
        let xp_at_50 = experience_gain(50, skill.knowledge, skill_def.difficulty);
        let xp_at_100 = experience_gain(100, skill.knowledge, skill_def.difficulty);

        assert_eq!(xp_at_0, 0); // Base: 0 * (1 + 0/0) = 0
        assert!(xp_at_50 >= 50);
        assert!(xp_at_100 >= 100);
    }

    #[test]
    fn test_level_calculation() {
        let skill_def = SkillRegistry::get_skill_by_id(SkillId::Swordsmanship).unwrap(); // Moderate difficulty = 4

        // L = min(floor(sqrt(XP/Diff)), 10)
        assert_eq!(level_from_experience(0, skill_def.difficulty), 0); // sqrt(0/4) = 0
        assert_eq!(level_from_experience(4, skill_def.difficulty), 1); // sqrt(4/4) = 1
        assert_eq!(level_from_experience(16, skill_def.difficulty), 2); // sqrt(16/4) = 2
        assert_eq!(level_from_experience(36, skill_def.difficulty), 3); // sqrt(36/4) = 3
        assert_eq!(level_from_experience(400, skill_def.difficulty), 10); // sqrt(400/4) = 10, capped at 10
        assert_eq!(level_from_experience(500, skill_def.difficulty), 10); // Still capped at 10
    }

    #[test]
    fn test_difficulty_levels() {
        assert_eq!(SkillDifficulty::VeryEasy.difficulty(), 1);
        assert_eq!(SkillDifficulty::Legendary.difficulty(), 32);
        assert_eq!(SkillDifficulty::Moderate.difficulty(), 4);
    }

    #[test]
    fn test_category_filtering() {
        let combat_skills = SkillRegistry::get_skills_by_category(SkillCategory::Combat);
        assert!(combat_skills.len() > 0);
        assert!(
            combat_skills
                .iter()
                .all(|s| s.category == SkillCategory::Combat)
        );
    }
}


