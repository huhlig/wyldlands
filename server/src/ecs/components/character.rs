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

//! Character components for stats, health, and progression

mod attributes;
pub mod builder;
mod macros;
mod nationality;
mod skills;
mod talents;

pub use self::attributes::{
    AttributeClass, AttributeScores, AttributeType, BodyAttributeScores, MindAttributeScores,
    SoulAttributeScores, chargen_attribute_cost, chargen_total_attribute_cost,
};
pub use self::builder::CharacterBuilder;
pub use self::nationality::Nationality;
pub use self::skills::{
    Skill, SkillCategory, SkillDifficulty, SkillEntry, Skills, chargen_skill_cost,
    chargen_total_skill_cost, skill_experience_floor_for_level, skill_knowledge_cap_for_level,
    skill_level_from_experience,
};
pub use self::talents::{
    Talent, TalentCategory, TalentEntry, Talents, talent_experience_floor_for_rank,
    talent_rank_from_experience,
};
use serde::{Deserialize, Serialize};

/// Indicates Entity Has a Memory, See [`MemoryResource`] for details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory;

impl Memory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body_attributes() {
        let attrs = AttributeScores::new();
        assert_eq!(attrs.score_offence, 10);
        assert_eq!(attrs.health_maximum, 100.0);
    }

    #[test]
    fn test_skills() {
        let mut skills = Skills::new();
        skills.add_skill(Skill::Swordsmanship, 0, 0);

        // Add experience and check level calculation
        // Swordsmanship is Moderate difficulty (base XP = 20)
        // Level formula: L = min(floor(sqrt(XP/Diff)), 10)
        skills.improve(Skill::Swordsmanship, 80); // Total: 80
        assert_eq!(skills.get_experience(Skill::Swordsmanship).unwrap(), 80);
        assert_eq!(skills.level(Skill::Swordsmanship), 4); // floor(sqrt(80/4)) = floor(4.47) = 4

        // Test level cap at 10
        skills.improve(Skill::Swordsmanship, 2000); // Total: 2080
        assert_eq!(skills.level(Skill::Swordsmanship), 10); // Capped at 10
    }
}
