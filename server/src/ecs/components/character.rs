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
mod skill;

pub use self::attributes::{BodyAttributes, MindAttributes, SoulAttributes};
pub use self::skill::{
    SKILL_REGISTRY, Skill, SkillCategory, SkillDefinition, SkillDifficulty, SkillId, Skills,
};

// Re-export shared character builder types from common
pub use wyldlands_common::character::{
    AttributeType, CharacterBuilder, StartingLocation, Talent,
    attribute_cost, skill_cost, total_attribute_cost, total_skill_cost,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body_attributes() {
        let attrs = BodyAttributes::new();
        assert_eq!(attrs.score_offence, 10);
        assert_eq!(attrs.health_maximum, 100.0);
    }

    #[test]
    fn test_skills() {
        let mut skills = Skills::new();
        skills.set(
            SkillId::Swordsmanship,
            Skill {
                experience: 0,
                knowledge: 0,
            },
        );

        // Add experience and check level calculation
        // Swordsmanship is Moderate difficulty (base XP = 20)
        // Level formula: L = min(floor(sqrt(XP/Diff)), 10)
        skills.improve(SkillId::Swordsmanship, 80); // Total: 80
        let skill = skills.get(SkillId::Swordsmanship).unwrap();
        assert_eq!(skills.level(SkillId::Swordsmanship), 4); // floor(sqrt(80/4)) = floor(4.47) = 4
        assert_eq!(skill.experience, 80);

        // Test level cap at 10
        skills.improve(SkillId::Swordsmanship, 2000); // Total: 2080
        assert_eq!(skills.level(SkillId::Swordsmanship), 10); // Capped at 10
    }
}


