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

//! # Skills
//!
//! ## Entity Component
//! Skills are represented as a blobk by the [Skills] Component. Within the component is a map of
//! [`SkillId`] -> ['Skill'] which contains the characters current experience and knowledge.
//!
//! Maps to: `entity_skills` table in database (one row per skill)
//!
//! ## Skill Levels
//! * **No Skill – Untrained**
//!   _You fumble through the motions, lacking even the basics. Every attempt is clumsy, uncertain, and prone to failure._
//! * ***Level 1 – Apprentice**
//!   _You have taken your first steps under guidance. You can mimic techniques but rely heavily on instruction to succeed._
//! * ***Level 2 – Novice**
//!   _You grasp the fundamentals and can act without constant supervision, though your movements still carry hesitation and error._
//! * ***Level 3 – Initiate**
//!   _The principles have become familiar. You begin to understand the “why” behind the motions, laying the foundation for true growth._
//! * **Level 4 – Adept**
//!   _Skill now flows with confidence. You handle common challenges with competence and surprise others with flashes of talent._
//! * **Level 5 – Journeyman**
//!   _Seasoned through practice and repetition, you perform reliably in all but the most trying circumstances. Your craft is trusted._
//! * **Level 6 – Master**
//!   _Your skill is unmistakable. With precision and control, you shape outcomes rather than merely respond to them. Others look to you for teaching._
//! * **Level 7 – Expert**
//!   _Your mastery is honed to brilliance. You see patterns invisible to most, executing techniques with effortless excellence._
//! * **Level 8 – Paragon**
//!   _You are the living embodiment of your art. Your every action demonstrates flawless form, setting the standard by which all others are judged._
//! * **Level 9 – Mythical**
//!   _Stories struggle to capture your feats. Mortals whisper your name in awe, for your abilities defy reason and seem touched by the divine._
//! * **Level 10 – Legendary**
//!   _Your skill has transcended the bounds of time and mortality. You are not merely remembered—you are enshrined in myth, an eternal icon of perfection._
//!
//! ## Skill Difficulty
//! [`SkillDifficulty`] is determined by the base experience gain (B) and the skill's inherent
//! complexity. Higher difficulty skills require more experience to level up.
//!
//! | Difficulty | Description                       | Xp Rate | Knowledge Rate |
//! |------------|-----------------------------------|---------|----------------|
//! | Very Easy  | Quick to learn, low skill ceiling | 2.00    | 2.00           |
//! | Easy       | Straightforward to learn          | 1.50    | 1.50           |
//! | Moderate   | Standard difficulty               | 1.00    | 1.00           |
//! | Hard       | Challenging to master             | 0.75    | 0.75           |
//! | Very Hard  | Extremely difficult to master     | 0.50    | 0.50           |
//! | Legendary  | Near impossible to fully master   | 0.25    | 0.25           |
//!
//! ## Skill Category
//! [`SkillCategory`] is used to group skills together for UI and organizational purposes.
//!
//! ## Mechanics
//! ### **Experience Gain Formula**
//!
//! ΔE = B * (1 + K/M)
//!
//! Where:
//! - **ΔE** = Experience gained per action
//! - **B** = Base experience gain (determined by difficulty of the action)
//! - **K** = Current Knowledge level of the skill
//! - **M** = Maximum possible Knowledge level for that skill (acts as a normalization factor)
//!
//! ### **How It Works**
//! - At **K = 0**, the character still gains Experience but at the slowest possible rate (**just the base amount**).
//! - As **Knowledge increases**, Experience gain speeds up.
//! - When **\(K = M\)**, Experience is gained at **double the base rate**.
//! - This creates a system where **higher Knowledge accelerates learning by up to 2x**, but even without Knowledge, Experience can still increase—just very slowly.
//!
//! ### **Equations**
//! L = Level `0-10`
//! L = min(floor(sqrt(XP/Diff)), 10)
//!
//! XP(L) = Maximum Experience for that level
//! XP(L) = M = A * L^2
//!
//! ΔE = Experience gained per action
//! ΔE = B * (1 + K/M)
//!
//! A = Skill Difficulty Experience Gain Rate
//! B = XP Difficulty Rate
//! K = Current Knowledge
//! M = Knowledge Cap for current level

use crate::define_skills;
use crate::ecs::components::{Talent, Talents};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

define_skills! {
    // Combat Skills
    Swordsmanship {
        name: "Swordsmanship",
        description: "The art of wielding swords in combat",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
     Axemanship{
        name: "Axemanship",
        description: "Proficiency with axes and similar chopping weapons",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Spearmanship {
        name: "Spearmanship",
        description: "Skill with spears, polearms, and reach weapons",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Archery {
        name: "Archery",
        description: "Precision and power with bows",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
     Crossbows{
        name: "Crossbows",
        description: "Operating and maintaining crossbows",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Easy,
        requires: None,
        cost: Some(1),
    },
    Daggers {
        name: "Daggers",
        description: "Quick strikes with small blades",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Easy,
        requires: None,
        cost: Some(1),
    },

    Unarmed {
        name: "Unarmed Combat",
        description: "Fighting without weapons using fists, kicks, and grappling",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Shields {
        name: "Shields",
        description: "Defensive techniques with shields",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Parrying {
        name: "Parrying",
        description: "Deflecting attacks with weapons",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Dodging {
        name: "Dodging",
        description: "Avoiding attacks through agility",
        category: SkillCategory::Combat,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    // Magic Skills
    Evocation {
        name: "Evocation",
        description: "Channeling raw magical energy into destructive force",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::Hard,
        requires: Some(Talent::Channeler),
        cost: Some(1),
    },
    Conjuration {
        name: "Conjuration",
        description: "Summoning creatures and objects from other planes",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::VeryHard,
        requires: Some(Talent::Channeler),
        cost: Some(1),
    },
    Illusion {
        name: "Illusion",
        description: "Creating false sensory experiences",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::Hard,
        requires: Some(Talent::Channeler),
        cost: Some(1),
    },
    Enchantment {
        name: "Enchantment",
        description: "Influencing minds and imbuing objects with magic",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::VeryHard,
        requires: Some(Talent::Channeler),
        cost: Some(1),
    },
    Divination {
        name: "Divination",
        description: "Perceiving hidden truths and future events",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::Legendary,
        requires: Some(Talent::Channeler),
        cost: Some(1),
    },
    Necromancy {
        name: "Necromancy",
        description: "Manipulating life force and commanding the undead",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::VeryHard,
        requires: Some(Talent::Channeler),
        cost: Some(1),
    },
    Transmutation {
        name: "Transmutation",
        description: "Altering the physical properties of matter",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::Hard,
        requires: Some(Talent::Channeler),
        cost: Some(1),
    },
    Abjuration {
        name: "Abjuration",
        description: "Creating protective barriers and dispelling magic",
        category: SkillCategory::Magic,
        difficulty: SkillDifficulty::Hard,
        requires: Some(Talent::Channeler),
        cost: Some(1),
    },

    // Crafting Skills
    Blacksmithing {
        name: "Blacksmithing",
        description: "Forging metal items at the anvil",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Armorsmithing {
        name: "Armorsmithing",
        description: "Crafting protective armor",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Weaponsmithing {
        name: "Weaponsmithing",
        description: "Forging weapons of war",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::VeryHard,
        requires: None,
        cost: Some(1),
    },
    Leatherworking {
        name: "Leatherworking",
        description: "Working with leather and hides",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Tailoring {
        name: "Tailoring",
        description: "Sewing cloth garments and items",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Easy,
        requires: None,
        cost: Some(1),
    },
    Jewelcrafting {
        name: "Jewelcrafting",
        description: "Creating jewelry and cutting gems",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Alchemy {
        name: "Alchemy",
        description: "Brewing potions and elixirs",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Cooking {
        name: "Cooking",
        description: "Preparing food and beverages",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Easy,
        requires: None,
        cost: Some(1),
    },
    Carpentry {
        name: "Carpentry",
        description: "Working with wood to create structures and items",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Masonry {
        name: "Masonry",
        description: "Building with stone and brick",
        category: SkillCategory::Crafting,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Mining {
        name: "Mining",
        description: "Extracting ore and minerals from the earth",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Herbalism {
        name: "Herbalism",
        description: "Gathering and identifying plants",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Skinning {
        name: "Skinning",
        description: "Harvesting hides and pelts from creatures",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::Easy,
        requires: None,
        cost: Some(1),
    },
    Fishing {
        name: "Fishing",
        description: "Catching fish from water",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::Easy,
        requires: None,
        cost: Some(1),
    },
    Logging {
        name: "Logging",
        description: "Felling trees and harvesting wood",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::Easy,
        requires: None,
        cost: Some(1),
    },
    Foraging {
        name: "Foraging",
        description: "Finding food and useful items in the wild",
        category: SkillCategory::Gathering,
        difficulty: SkillDifficulty::VeryEasy,
        requires: None,
        cost: Some(1),
    },

    // Social Skills
    Persuasion {
        name: "Persuasion",
        description: "Convincing others through charm and reason",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Intimidation {
        name: "Intimidation",
        description: "Influencing others through threats and fear",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Deception {
        name: "Deception",
        description: "Lying and misleading others convincingly",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Insight {
        name: "Insight",
        description: "Reading people's intentions and emotions",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Performance {
        name: "Performance",
        description: "Entertaining others through art and showmanship",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Leadership {
        name: "Leadership",
        description: "Inspiring and commanding others",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::VeryHard,
        requires: None,
        cost: Some(1),
    },
    Bartering {
        name: "Bartering",
        description: "Negotiating favorable trades and prices",
        category: SkillCategory::Social,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },

    // Survival Skills
    Tracking {
        name: "Tracking",
        description: "Following trails and reading signs",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Stealth {
        name: "Stealth",
        description: "Moving silently and remaining hidden",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Lockpicking {
        name: "Lockpicking",
        description: "Opening locks without keys",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Trapping {
        name: "Trapping",
        description: "Setting and disarming traps",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    FirstAid {
        name: "First Aid",
        description: "Treating wounds and ailments in the field",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Navigation {
        name: "Navigation",
        description: "Finding your way using landmarks and tools",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    AnimalHandling {
        name: "Animal Handling",
        description: "Training and controlling animals",
        category: SkillCategory::Survival,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },

    // Knowledge Skills
    History {
        name: "History",
        description: "Knowledge of past events and civilizations",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Arcana {
        name: "Arcana",
        description: "Understanding of magical theory and practice",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
    Nature {
        name: "Nature",
        description: "Knowledge of flora, fauna, and natural phenomena",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Religion {
        name: "Religion",
        description: "Understanding of deities, faiths, and rituals",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::Moderate,
        requires: None,
        cost: Some(1),
    },
    Medicine {
        name: "Medicine",
        description: "Advanced healing and anatomical knowledge",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::VeryHard,
        requires: None,
        cost: Some(1),
    },
    Engineering {
        name: "Engineering",
        description: "Understanding of mechanisms and construction",
        category: SkillCategory::Knowledge,
        difficulty: SkillDifficulty::Hard,
        requires: None,
        cost: Some(1),
    },
}

/// Individual Skill Entry for a Character
/// Maps to: entity_skills table (one row per skill)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SkillEntry {
    /// The unique identifier for this skill.
    pub skill: Skill,
    /// Current experience points in this skill.
    pub experience: i32,
    /// Current knowledge level in this skill, which accelerates experience gain.
    pub knowledge: i32,
}

/// Character skills collection Component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skills(HashMap<Skill, SkillEntry>);

impl Skills {
    /// Create a new empty skill set
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Check if a skill exists in the collection.
    pub fn has_skill(&self, skill: Skill) -> bool {
        self.0.contains_key(&skill)
    }

    /// Get Experience for a Skill
    pub fn get_experience(&self, skill: Skill) -> Option<i32> {
        self.0.get(&skill).map(|entry| entry.experience)
    }

    /// Get Knowledge for a Skill
    pub fn get_knowledge(&self, skill: Skill) -> Option<i32> {
        self.0.get(&skill).map(|entry| entry.knowledge)
    }

    /// Add a skill to the collection.
    pub fn add_skill(&mut self, skill: Skill, experience: i32, knowledge: i32) {
        self.0.insert(
            skill,
            SkillEntry {
                skill,
                experience,
                knowledge,
            },
        );
    }

    /// Remove a skill from the collection.
    pub fn remove_skill(&mut self, skill: Skill) {
        self.0.remove(&skill);
    }

    /// Advance a skill by experience and knowledge.
    pub fn advance(&mut self, skill: Skill, experience: i32, knowledge: i32) {
        if let Some(skill) = self.0.get_mut(&skill) {
            skill.experience += experience;
            skill.knowledge += knowledge;
        }
    }

    /// Get the current level of a skill (0-10).
    ///
    /// The level is calculated based on experience and difficulty:
    /// `L = min(floor(sqrt(XP/D)), 10)` where `D` is the difficulty scalar.
    pub fn level(&self, skill: Skill) -> i32 {
        if let Some(entry) = self.0.get(&skill) {
            skill_level_from_experience(entry.experience, skill.difficulty())
        } else {
            0
        }
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over all skills, levels, experience, and knowledge.
    pub fn iter(&self) -> impl Iterator<Item = (Skill, i32, i32, i32)> + '_ {
        self.0.iter().map(|(skill, entry)| {
            (
                entry.skill,
                self.level(entry.skill),
                entry.experience,
                entry.knowledge,
            )
        })
    }

    /// Get the number of skills in the collection.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Get the knowledge cap for the current level of a skill.
    ///
    /// The knowledge cap determines the maximum knowledge a character can have at their current skill level.
    pub fn knowledge_cap(&self, skill: Skill) -> i32 {
        skill_knowledge_cap_for_level(self.level(skill), skill.difficulty())
    }

    /// Improve a skill by adding base experience points.
    ///
    /// The actual experience gained is accelerated by current knowledge:
    /// `ΔE = B * (1 + K/M)`
    pub fn improve(&mut self, skill: Skill, points: i32) {
        let current_level = self.level(skill);
        if let Some(entry) = self.0.get_mut(&skill) {
            let knowledge_cap =
                skill_knowledge_cap_for_level(current_level, entry.skill.difficulty());
            if knowledge_cap == 0 {
                entry.experience += points;
            } else {
                entry.experience += points * (1 + entry.knowledge / knowledge_cap);
            }
        }
    }

    /// Train a skill by adding knowledge points.
    ///
    /// Knowledge is capped by the knowledge cap of the next level.
    pub fn train(&mut self, skill: Skill, points: i32) {
        let next_level = 1 + self.level(skill);
        if let Some(entry) = self.0.get_mut(&skill) {
            // Calculate knowledge cap for next level.
            let knowledge_cap = skill_knowledge_cap_for_level(next_level, entry.skill.difficulty());
            let knowledge = entry.knowledge + (points * (1 + entry.knowledge / knowledge_cap));
            entry.knowledge = i32::min(knowledge, knowledge_cap);
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
    /// Iterator over SkillDifficulty variants
    pub fn iter() -> impl Iterator<Item = Self> {
        [
            SkillDifficulty::VeryEasy,
            SkillDifficulty::Easy,
            SkillDifficulty::Moderate,
            SkillDifficulty::Hard,
            SkillDifficulty::VeryHard,
            SkillDifficulty::Legendary,
        ]
        .into_iter()
    }
    /// Skill Difficulty Name
    pub fn name(&self) -> &'static str {
        match self {
            SkillDifficulty::VeryEasy => "Very Easy",
            SkillDifficulty::Easy => "Easy",
            SkillDifficulty::Moderate => "Moderate",
            SkillDifficulty::Hard => "Hard",
            SkillDifficulty::VeryHard => "Very Hard",
            SkillDifficulty::Legendary => "Legendary",
        }
    }
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

impl std::fmt::Display for SkillDifficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Skill categories for organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillCategory {
    /// Martial and defensive skills.
    Combat,
    /// Arcane and supernatural skills.
    Magic,
    /// Item creation and repair skills.
    Crafting,
    /// Resource acquisition skills.
    Gathering,
    /// Interpersonal and influence skills.
    Social,
    /// Wilderness and environmental survival skills.
    Survival,
    /// Academic and theoretical knowledge.
    Knowledge,
    /// Mental powers and psychic abilities.
    Psionic,
    /// Ethereal and cosmic knowledge.
    Akashic,
}

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
///
/// Get Level from Experience (0-10)
///
/// ## Equation
/// L = min(floor(sqrt(XP/Diff)), 10)
pub fn skill_level_from_experience(exp: i32, difficulty: SkillDifficulty) -> i32 {
    i32::min(
        f32::floor(f32::sqrt(exp as f32 / difficulty.difficulty() as f32)) as i32,
        10,
    )
}

/// Get Experience floor for a given level
///
/// ## Equation
/// XP(L) = Diff * L^2
///
/// See [`skill_level_from_experience`]
pub fn skill_experience_floor_for_level(level: i32, difficulty: SkillDifficulty) -> i32 {
    difficulty.difficulty() * level.pow(2)
}

/// Knowledge Cap for level
///
/// ## Equation
/// XP(L) = M = (A * (L+1)^2) -1
pub fn skill_knowledge_cap_for_level(level: i32, difficulty: SkillDifficulty) -> i32 {
    (difficulty.difficulty() * (level + 1).pow(2)) - 1
}

/// Experience gained per action
///
/// ## Equation
/// ΔE = B * (1 + K/M)
fn experience_gain(points: i32, knowledge: i32, difficulty: SkillDifficulty) -> i32 {
    let cap =
        skill_knowledge_cap_for_level(skill_level_from_experience(points, difficulty), difficulty);
    if cap == 0 {
        points
    } else {
        points * (1 + knowledge / cap)
    }
}

/// Point costs for skill ranks (progressive cost)
/// Rank 1-3: 1 point each
/// Rank 4-6: 2 points each
/// Rank 7-9: 3 points each
/// Rank 10: 4 points
pub fn chargen_skill_cost(rank: i32) -> i32 {
    match rank {
        1..=3 => 1,
        4..=6 => 2,
        7..=9 => 3,
        10 => 4,
        _ => 0,
    }
}

/// Calculate total cost for a skill at a given rank
pub fn chargen_total_skill_cost(rank: i32) -> i32 {
    (1..=rank).map(chargen_skill_cost).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_scaling() {
        for difficulty in SkillDifficulty::iter() {
            println!("Difficulty: {}", difficulty);
            for rank in 0..10 {
                let exp_floor = skill_knowledge_cap_for_level(1, difficulty);
                let cap_next = skill_knowledge_cap_for_level(2, difficulty);
                println!(
                    "    Rank {:2?}, Exp: {:8}, Cap: {:8}",
                    rank, exp_floor, cap_next
                );
            }
        }
    }

    #[test]
    fn test_experience_calculation() {
        let entry = SkillEntry {
            skill: Skill::Swordsmanship,
            experience: 0,
            knowledge: 0,
        };
        let xp_at_0 = experience_gain(0, entry.knowledge, entry.skill.difficulty());
        let xp_at_50 = experience_gain(50, entry.knowledge, entry.skill.difficulty());
        let xp_at_100 = experience_gain(100, entry.knowledge, entry.skill.difficulty());

        assert_eq!(xp_at_0, 0); // Base: 0 * (1 + 0/0) = 0
        assert!(xp_at_50 >= 50);
        assert!(xp_at_100 >= 100);
    }

    #[test]
    fn test_level_calculation() {
        let skill = Skill::Swordsmanship; // Moderate difficulty = 4

        // L = min(floor(sqrt(XP/Diff)), 10)
        assert_eq!(skill_level_from_experience(0, skill.difficulty()), 0); // sqrt(0/4) = 0
        assert_eq!(skill_level_from_experience(4, skill.difficulty()), 1); // sqrt(4/4) = 1
        assert_eq!(skill_level_from_experience(16, skill.difficulty()), 2); // sqrt(16/4) = 2
        assert_eq!(skill_level_from_experience(36, skill.difficulty()), 3); // sqrt(36/4) = 3
        assert_eq!(skill_level_from_experience(400, skill.difficulty()), 10); // sqrt(400/4) = 10, capped at 10
        assert_eq!(skill_level_from_experience(500, skill.difficulty()), 10); // Still capped at 10
    }

    #[test]
    fn test_difficulty_levels() {
        assert_eq!(SkillDifficulty::VeryEasy.difficulty(), 1);
        assert_eq!(SkillDifficulty::Legendary.difficulty(), 32);
        assert_eq!(SkillDifficulty::Moderate.difficulty(), 4);
    }

    #[test]
    fn test_experience_floor_calculation() {
        let skill = Skill::Swordsmanship; // Moderate difficulty = 4

        assert_eq!(skill_experience_floor_for_level(0, skill.difficulty()), 0);
        assert_eq!(skill_experience_floor_for_level(1, skill.difficulty()), 4);
        assert_eq!(skill_experience_floor_for_level(2, skill.difficulty()), 16);
        assert_eq!(skill_experience_floor_for_level(3, skill.difficulty()), 36);
        assert_eq!(
            skill_experience_floor_for_level(10, skill.difficulty()),
            400
        );

        // Verify inverse relationship
        for level in 0..=10 {
            let xp = skill_experience_floor_for_level(level, skill.difficulty());
            assert_eq!(skill_level_from_experience(xp, skill.difficulty()), level);
        }
    }
}
