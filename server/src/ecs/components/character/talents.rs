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

use crate::define_talents;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

define_talents! {
    WeaponMaster {
        name: "Weapon Master",
        description: "Expertise in multiple weapon types.",
        category: TalentCategory::Combat,
        requires: None,
        cost: Some(5),
    },
    ShieldExpert {
        name: "Shield Expert",
        description: "Enhanced effectiveness with shields.",
        category: TalentCategory::Combat,
        requires: None,
        cost: Some(5),
    },
    DualWielder {
        name: "Dual Wielder",
        description: "Ability to fight with two weapons effectively.",
        category: TalentCategory::Combat,
        requires: None,
        cost: Some(5),
    },
    Berserker {
        name: "Berserker",
        description: "Channeling rage for combat power.",
        category: TalentCategory::Combat,
        requires: None,
        cost: Some(5),
    },
    Tactician {
        name: "Tactician",
        description: "Strategic awareness on the battlefield.",
        category: TalentCategory::Combat,
        requires: None,
        cost: Some(5),
    },
    Spellweaver {
        name: "Spellweaver",
        description: "Proficiency in weaving complex spells.",
        category: TalentCategory::Magic,
        requires: None,
        cost: Some(5),
    },
    ElementalAffinity {
        name: "Elemental Affinity",
        description: "Innate connection to elemental forces.",
        category: TalentCategory::Magic,
        requires: None,
        cost: Some(5),
    },
    ArcaneScholar {
        name: "Arcane Scholar",
        description: "Deep understanding of arcane principles.",
        category: TalentCategory::Magic,
        requires: None,
        cost: Some(5),
    },
    Ritualist {
        name: "Ritualist",
        description: "Expertise in performing magical rituals.",
        category: TalentCategory::Magic,
        requires: None,
        cost: Some(5),
    },
    Channeler {
        name: "Channeler",
        description: "Ability to channel large amounts of magical energy.",
        category: TalentCategory::Magic,
        requires: None,
        cost: Some(5),
    },
    AstralProjection {
        name: "Astral Projection",
        description: "Skill in projecting one's spirit outside the body.",
        category: TalentCategory::Magic,
        requires: None,
        cost: Some(5),
    },
    MasterCraftsman {
        name: "Master Craftsman",
        description: "Exceptional skill in manual crafting.",
        category: TalentCategory::Crafting,
        requires: None,
        cost: Some(3),
    },
    Artificer {
        name: "Artificer",
        description: "Skill in creating complex mechanical or magical items.",
        category: TalentCategory::Crafting,
        requires: None,
        cost: Some(3),
    },
    Alchemist {
        name: "Alchemist",
        description: "Expertise in brewing potions and concoctions.",
        category: TalentCategory::Crafting,
        requires: None,
        cost: Some(3),
    },
    Enchanter {
        name: "Enchanter",
        description: "Skill in imbuing items with magical properties.",
        category: TalentCategory::Crafting,
        requires: None,
        cost: Some(3),
    },
    SilverTongue {
        name: "Silver Tongue",
        description: "Exceptional persuasive ability.",
        category: TalentCategory::Social,
        requires: None,
        cost: Some(5),
    },
    NaturalLeader {
        name: "Natural Leader",
        description: "Innate ability to inspire and lead others.",
        category: TalentCategory::Social,
        requires: None,
        cost: Some(5),
    },
    Intimidating {
        name: "Intimidating",
        description: "Presence that commands respect or fear.",
        category: TalentCategory::Social,
        requires: None,
        cost: Some(5),
    },
    Empathic {
        name: "Empathic",
        description: "Understanding of others' emotions.",
        category: TalentCategory::Social,
        requires: None,
        cost: Some(5),
    },
    Streetwise {
        name: "Streetwise",
        description: "Knowledge of the urban underworld.",
        category: TalentCategory::Social,
        requires: None,
        cost: Some(5),
    },
    Woodsman {
        name: "Woodsman",
        description: "Expertise in surviving in the wilderness.",
        category: TalentCategory::Survival,
        requires: None,
        cost: Some(5),
    },
    Tracker {
        name: "Tracker",
        description: "Ability to follow trails through any terrain.",
        category: TalentCategory::Survival,
        requires: None,
        cost: Some(5),
    },
    Scout {
        name: "Scout",
        description: "Expertise in reconnaissance and stealth.",
        category: TalentCategory::Survival,
        requires: None,
        cost: Some(5),
    },
    Hardy {
        name: "Hardy",
        description: "Increased resilience to environmental hazards.",
        category: TalentCategory::Survival,
        requires: None,
        cost: Some(5),
    },
    Forager {
        name: "Forager",
        description: "Aptitude for finding food and water.",
        category: TalentCategory::Survival,
        requires: None,
        cost: Some(5),
    },
    Lucky {
        name: "Lucky",
        description: "Fortune seems to favor you in all things.",
        category: TalentCategory::Special,
        requires: None,
        cost: Some(7),
    },
    Ambidextrous {
        name: "Ambidextrous",
        description: "Equal proficiency with both hands.",
        category: TalentCategory::Special,
        requires: None,
        cost: Some(7),
    },
    FastLearner {
        name: "Fast Learner",
        description: "Ability to learn new skills and talents faster.",
        category: TalentCategory::Special,
        requires: None,
        cost: Some(7),
    },
    Resilient {
        name: "Resilient",
        description: "Exceptional mental and physical toughness.",
        category: TalentCategory::Special,
        requires: None,
        cost: Some(7),
    },
}

/// An individual talent acquired by a character.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TalentEntry {
    /// The unique identifier of the talent.
    pub talent: Talent,
    /// Current experience points in this talent.
    pub experience: i32,
}

impl TalentEntry {
    /// Create a new Talent with specified ID and initial experience
    pub fn new(talent: Talent, experience: i32) -> TalentEntry {
        Self { talent, experience }
    }
    /// Get Current Talent Rank
    pub fn get_rank(&self) -> u8 {
        talent_rank_from_experience(self.experience)
    }

    /// Get Current Talent Experience
    pub fn get_experience(&self) -> i32 {
        self.experience
    }

    /// Add Experience to Talent
    pub fn add_experience(&mut self, experience: i32) {
        self.experience += experience;
    }
}

/// Category of talents
#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TalentCategory {
    Combat,
    Magic,
    Crafting,
    Social,
    Survival,
    Special,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Talents(BTreeMap<Talent, TalentEntry>);
impl Talents {
    /// Create a new empty collection of talents.
    pub fn new() -> Talents {
        Self(BTreeMap::new())
    }

    /// Check if the character has a specific talent.
    pub fn has_talent(&self, talent: Talent) -> bool {
        self.0.contains_key(&talent)
    }

    /// Add a talent to the character.
    pub fn add_talent(&mut self, talent: Talent, experience: i32) {
        self.0.insert(talent, TalentEntry { talent, experience });
    }

    /// Remove a talent from the character.
    pub fn remove_talent(&mut self, talent: Talent) {
        self.0.remove(&talent);
    }
    /// List all acquired talents along with their rank and experience.
    pub fn list_talents(&self) -> impl Iterator<Item = (Talent, u8, i32)> {
        self.0.iter().map(|(id, talent)| {
            (
                *id,
                talent_rank_from_experience(talent.experience),
                talent.experience,
            )
        })
    }

    /// Iterate over all acquired talents, with ranks and experience
    pub fn iter(&self) -> impl Iterator<Item = (Talent, u8, i32)> + '_ {
        self.0
            .values()
            .map(|talent| (talent.talent, talent.get_rank(), talent.experience))
    }
    /// Get the rank of a specific talent.
    pub fn get_talent_rank(&self, talent: Talent) -> Option<u8> {
        self.0.get(&talent).map(|entry| entry.get_rank())
    }
    /// Get the experience points of a specific talent.
    pub fn get_talent_experience(&self, talent: Talent) -> Option<i32> {
        self.0.get(&talent).map(|entry| entry.get_experience())
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the number of talents in the collection.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Add experience to a specific talent.
    pub fn add_talent_experience(&mut self, talent: Talent, experience: i32) {
        self.0
            .get_mut(&talent)
            .map(|entry| entry.add_experience(experience));
    }
}

/// Growth Factor
const A: f32 = 1.0;
/// Scale Constant
const C: f32 = 1.0;
/// Euler's Number
const E: f32 = std::f32::consts::E;

/// Calculate Rank based on Experience
pub fn talent_rank_from_experience(experience: i32) -> u8 {
    (f32::floor(A * f32::ln((experience as f32 / C) + 1.0)) + 1.0) as u8
}

/// Calculate Experience Floor for a given Rank
pub fn talent_experience_floor_for_rank(rank: u8) -> i32 {
    (C * (E * (rank as f32 / A) - 1.0)) as i32
}
