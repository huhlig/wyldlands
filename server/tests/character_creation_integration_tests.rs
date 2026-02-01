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

//! Integration tests for character creation flow
//!
//! These tests verify the complete character creation process from
//! session authentication through character finalization.

use std::str::FromStr;
use wyldlands_server::ecs::components::{AttributeType, CharacterBuilder, Talent, Skill};

#[test]
fn test_character_builder_creation() {
    let builder = CharacterBuilder::new("TestChar".to_string(), 50, 30);

    assert_eq!(builder.name, "TestChar");
    assert_eq!(builder.attribute_talent_points, 50);
    assert_eq!(builder.skill_points, 30);
    assert_eq!(builder.max_attribute_talent_points, 50);
    assert_eq!(builder.max_skill_points, 30);

    // All attributes should start at 10
    for attr in AttributeType::all() {
        assert_eq!(builder.get_attribute(attr), 10);
    }

    // No talents or skills initially
    assert!(builder.talents.is_empty());
    assert!(builder.skills.is_empty());
}

#[test]
fn test_character_creation_full_flow() {
    let mut builder = CharacterBuilder::new("Warrior".to_string(), 50, 30);

    // Set starting location
    builder.set_starting_location("start_room_001".to_string());

    // Increase combat attributes
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, 3)
            .is_ok()
    );
    assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 13);

    assert!(
        builder
            .modify_attribute(AttributeType::BodyDefence, 2)
            .is_ok()
    );
    assert_eq!(builder.get_attribute(AttributeType::BodyDefence), 12);

    // Add combat talents
    assert!(
        builder
            .modify_talent(Talent::WeaponMaster, true)
            .is_ok()
    );
    assert!(
        builder
            .modify_talent(Talent::ShieldExpert, true)
            .is_ok()
    );
    assert_eq!(builder.talents.len(), 2);

    // Add combat skills
    assert!(builder.modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 5).is_ok());
    assert_eq!(builder.get_skill_level(Skill::from_str("Swordsmanship").unwrap()), 5);

    assert!(builder.modify_skill_points(Skill::from_str("Shields").unwrap(), 3).is_ok());
    assert_eq!(builder.get_skill_level(Skill::from_str("Shields").unwrap()), 3);

    // Validate character
    assert!(builder.validate().is_ok());
    assert!(builder.is_valid());
}

#[test]
fn test_character_creation_mage_build() {
    let mut builder = CharacterBuilder::new("Mage".to_string(), 50, 30);
    builder.set_starting_location("start_room_001".to_string());

    // Increase magic attributes
    assert!(
        builder
            .modify_attribute(AttributeType::MindOffence, 4)
            .is_ok()
    );
    assert!(
        builder
            .modify_attribute(AttributeType::MindFinesse, 3)
            .is_ok()
    );
    assert!(
        builder
            .modify_attribute(AttributeType::SoulOffence, 2)
            .is_ok()
    );

    // Add magic talents
    assert!(
        builder
            .modify_talent(Talent::Spellweaver, true)
            .is_ok()
    );
    assert!(
        builder
            .modify_talent(Talent::ElementalAffinity, true)
            .is_ok()
    );
    assert!(
        builder
            .modify_talent(Talent::ArcaneScholar, true)
            .is_ok()
    );

    // Add magic skills
    assert!(builder.modify_skill_points(Skill::from_str("Evocation").unwrap(), 6).is_ok());
    assert!(builder.modify_skill_points(Skill::from_str("Abjuration").unwrap(), 4).is_ok());
    assert!(builder.modify_skill_points(Skill::from_str("Divination").unwrap(), 2).is_ok());

    // Validate
    assert!(builder.validate().is_ok());
}

#[test]
fn test_character_creation_validation_errors() {
    // Test empty name
    let builder = CharacterBuilder::new("".to_string(), 50, 30);
    assert!(builder.validate().is_err());
    let errors = builder.validation_errors();
    assert!(errors.iter().any(|e| e.contains("name")));

    // Test missing starting location
    let builder = CharacterBuilder::new("Test".to_string(), 50, 30);
    assert!(builder.validate().is_err());
    let errors = builder.validation_errors();
    assert!(errors.iter().any(|e| e.contains("Starting location")));

    // Test overspending points
    let mut builder = CharacterBuilder::new("Test".to_string(), 10, 10);
    builder.set_starting_location("start_room_001".to_string());

    // Try to spend more than available
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, 5)
            .is_err()
    );
}

#[test]
fn test_attribute_modification_costs() {
    let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

    let initial_points = builder.attribute_talent_points;

    // Increase from 10 to 11 costs 3 points (rank 11 is in 11-15 range)
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, 1)
            .is_ok()
    );
    assert_eq!(builder.attribute_talent_points, initial_points - 3);

    // Increase from 11 to 12 costs 3 points (rank 12 is in 11-15 range)
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, 1)
            .is_ok()
    );
    assert_eq!(builder.attribute_talent_points, initial_points - 3 - 3);

    // Decrease from 12 to 11 refunds 3 points
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, -1)
            .is_ok()
    );
    assert_eq!(builder.attribute_talent_points, initial_points - 3);

    // Decrease from 11 to 10 refunds 3 points
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, -1)
            .is_ok()
    );
    assert_eq!(builder.attribute_talent_points, initial_points);
}

#[test]
fn test_attribute_limits() {
    let mut builder = CharacterBuilder::new("Test".to_string(), 1000, 30);

    // Cannot go below 10
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, -1)
            .is_err()
    );
    assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 10);

    // Can increase to 20
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, 10)
            .is_ok()
    );
    assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 20);

    // Cannot exceed 20
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, 1)
            .is_err()
    );
    assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 20);
}

#[test]
fn test_talent_modification() {
    let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

    let initial_points = builder.attribute_talent_points;

    // Add talent (costs 5 points)
    assert!(
        builder
            .modify_talent(Talent::WeaponMaster, true)
            .is_ok()
    );
    assert_eq!(builder.talents.len(), 1);
    assert_eq!(builder.attribute_talent_points, initial_points - 5);

    // Cannot add same talent twice
    assert!(
        builder
            .modify_talent(Talent::WeaponMaster, true)
            .is_err()
    );
    assert_eq!(builder.talents.len(), 1);

    // Remove talent (refunds 5 points)
    assert!(
        builder
            .modify_talent(Talent::WeaponMaster, false)
            .is_ok()
    );
    assert_eq!(builder.talents.len(), 0);
    assert_eq!(builder.attribute_talent_points, initial_points);

    // Cannot remove talent that wasn't added
    assert!(
        builder
            .modify_talent(Talent::ShieldExpert, false)
            .is_err()
    );
}

#[test]
fn test_skill_modification_costs() {
    let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

    let initial_points = builder.skill_points;

    // Increase from 0 to 1 costs 1 point
    assert!(builder.modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 1).is_ok());
    assert_eq!(builder.skill_points, initial_points - 1);

    // Increase from 1 to 2 costs 1 point
    assert!(builder.modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 1).is_ok());
    assert_eq!(builder.skill_points, initial_points - 2);

    // Decrease from 2 to 1 refunds 1 point
    assert!(
        builder
            .modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), -1)
            .is_ok()
    );
    assert_eq!(builder.skill_points, initial_points - 1);

    // Decrease from 1 to 0 refunds 1 point and removes skill
    assert!(
        builder
            .modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), -1)
            .is_ok()
    );
    assert_eq!(builder.skill_points, initial_points);
    assert!(!builder.skills.has_skill(Skill::from_str("Swordsmanship").unwrap()));
}

#[test]
fn test_skill_limits() {
    let mut builder = CharacterBuilder::new("Test".to_string(), 50, 100);

    // Cannot go below 0
    assert!(
        builder
            .modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), -1)
            .is_err()
    );

    // Can increase to 10
    assert!(
        builder
            .modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 10)
            .is_ok()
    );
    assert_eq!(builder.get_skill_level(Skill::from_str("Swordsmanship").unwrap()), 10);

    // Cannot exceed 10
    assert!(
        builder
            .modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 1)
            .is_err()
    );
    assert_eq!(builder.get_skill_level(Skill::from_str("Swordsmanship").unwrap()), 10);
}

#[test]
fn test_multiple_skills() {
    let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

    // Add multiple skills
    assert!(builder.modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 5).is_ok());
    assert!(builder.modify_skill_points(Skill::from_str("Archery").unwrap(), 3).is_ok());
    assert!(builder.modify_skill_points(Skill::from_str("Stealth").unwrap(), 2).is_ok());

    assert_eq!(builder.skills.len(), 3);
    assert_eq!(builder.get_skill_level(Skill::from_str("Swordsmanship").unwrap()), 5);
    assert_eq!(builder.get_skill_level(Skill::from_str("Archery").unwrap()), 3);
    assert_eq!(builder.get_skill_level(Skill::from_str("Stealth").unwrap()), 2);

    // Remove one skill
    assert!(builder.modify_skill_points(Skill::from_str("Archery").unwrap(), -3).is_ok());
    assert_eq!(builder.skills.len(), 2);
    assert!(!builder.skills.has_skill(Skill::from_str("Archery").unwrap()));
}

#[test]
fn test_point_pool_separation() {
    let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

    // Attributes and talents share a pool
    let initial_attr_talent = builder.attribute_talent_points;
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, 1)
            .is_ok()
    );
    assert!(
        builder
            .modify_talent(Talent::WeaponMaster, true)
            .is_ok()
    );
    assert_eq!(builder.attribute_talent_points, initial_attr_talent - 3 - 5);

    // Skills have a separate pool
    let initial_skill = builder.skill_points;
    // Skill ranks 1-3 cost 1 point each, ranks 4-5 cost 2 points each
    // So rank 1-5 costs: 1+1+1+2+2 = 7 points
    assert!(builder.modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 5).is_ok());
    assert_eq!(builder.skill_points, initial_skill - 7);

    // Skill spending doesn't affect attribute/talent pool
    assert_eq!(builder.attribute_talent_points, initial_attr_talent - 3 - 5);
}

#[test]
fn test_character_name_validation() {
    // Valid name
    let mut builder = CharacterBuilder::new("ValidName".to_string(), 50, 30);
    builder.set_starting_location("start_room_001".to_string());
    assert!(builder.validate().is_ok());

    // Empty name
    let mut builder = CharacterBuilder::new("".to_string(), 50, 30);
    builder.set_starting_location("start_room_001".to_string());
    assert!(builder.validate().is_err());

    // Name too long (over 50 characters)
    let long_name = "a".repeat(51);
    let mut builder = CharacterBuilder::new(long_name, 50, 30);
    builder.set_starting_location("start_room_001".to_string());
    let errors = builder.validation_errors();
    assert!(errors.iter().any(|e| e.contains("too long")));
}

#[test]
fn test_balanced_character_build() {
    let mut builder = CharacterBuilder::new("Balanced".to_string(), 50, 30);
    builder.set_starting_location("start_room_001".to_string());

    // Distribute points evenly across attributes
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, 2)
            .is_ok()
    );
    assert!(
        builder
            .modify_attribute(AttributeType::BodyFinesse, 2)
            .is_ok()
    );
    assert!(
        builder
            .modify_attribute(AttributeType::BodyDefence, 2)
            .is_ok()
    );
    assert!(
        builder
            .modify_attribute(AttributeType::MindOffence, 1)
            .is_ok()
    );
    assert!(
        builder
            .modify_attribute(AttributeType::MindFinesse, 1)
            .is_ok()
    );
    assert!(
        builder
            .modify_attribute(AttributeType::MindDefence, 1)
            .is_ok()
    );

    // Add versatile talents
    assert!(builder.modify_talent(Talent::Tactician, true).is_ok());
    assert!(builder.modify_talent(Talent::Alchemist, true).is_ok());

    // Add diverse skills
    assert!(builder.modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 3).is_ok());
    assert!(builder.modify_skill_points(Skill::from_str("Persuasion").unwrap(), 3).is_ok());
    assert!(builder.modify_skill_points(Skill::from_str("Insight").unwrap(), 3).is_ok());

    assert!(builder.validate().is_ok());
}

#[test]
fn test_min_max_character_builds() {
    // Minimum build - no modifications
    let mut builder = CharacterBuilder::new("Minimum".to_string(), 50, 30);
    builder.set_starting_location("start_room_001".to_string());
    assert!(builder.validate().is_ok());

    // Maximum build - spend all points
    let mut builder = CharacterBuilder::new("Maximum".to_string(), 100, 100);
    builder.set_starting_location("start_room_001".to_string());

    // Max out some attributes
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, 10)
            .is_ok()
    );
    assert!(
        builder
            .modify_attribute(AttributeType::MindOffence, 10)
            .is_ok()
    );

    // Add multiple talents
    assert!(
        builder
            .modify_talent(Talent::WeaponMaster, true)
            .is_ok()
    );
    assert!(
        builder
            .modify_talent(Talent::Spellweaver, true)
            .is_ok()
    );
    assert!(builder.modify_talent(Talent::Tactician, true).is_ok());

    // Max out multiple skills
    assert!(
        builder
            .modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 10)
            .is_ok()
    );
    assert!(builder.modify_skill_points(Skill::from_str("Evocation").unwrap(), 10).is_ok());

    assert!(builder.validate().is_ok());
}

#[test]
fn test_character_builder_clone() {
    let mut builder = CharacterBuilder::new("Original".to_string(), 50, 30);
    builder.set_starting_location("start_room_001".to_string());
    builder
        .modify_attribute(AttributeType::BodyOffence, 3)
        .unwrap();
    builder
        .modify_talent(Talent::WeaponMaster, true)
        .unwrap();
    builder
        .modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 5)
        .unwrap();

    // Clone the builder
    let cloned = builder.clone();

    // Verify all fields are copied
    assert_eq!(cloned.name, builder.name);
    assert_eq!(
        cloned.attribute_talent_points,
        builder.attribute_talent_points
    );
    assert_eq!(cloned.skill_points, builder.skill_points);
    assert_eq!(cloned.get_attribute(AttributeType::BodyOffence), 13);
    assert_eq!(cloned.talents.len(), 1);
    assert_eq!(cloned.get_skill_level(Skill::from_str("Swordsmanship").unwrap()), 5);
    assert_eq!(cloned.starting_location_id, builder.starting_location_id);
}

#[test]
fn test_edge_case_zero_points() {
    let mut builder = CharacterBuilder::new("ZeroPoints".to_string(), 0, 0);
    builder.set_starting_location("start_room_001".to_string());

    // Cannot modify anything with zero points
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, 1)
            .is_err()
    );
    assert!(
        builder
            .modify_talent(Talent::WeaponMaster, true)
            .is_err()
    );
    assert!(
        builder
            .modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 1)
            .is_err()
    );

    // But character is still valid (all attributes at base 10)
    assert!(builder.validate().is_ok());
}

#[test]
fn test_edge_case_negative_delta() {
    let mut builder = CharacterBuilder::new("Test".to_string(), 50, 30);

    // Cannot decrease from base value
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, -1)
            .is_err()
    );
    assert!(
        builder
            .modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), -1)
            .is_err()
    );

    // But can decrease after increasing
    builder
        .modify_attribute(AttributeType::BodyOffence, 2)
        .unwrap();
    assert!(
        builder
            .modify_attribute(AttributeType::BodyOffence, -1)
            .is_ok()
    );

    builder
        .modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), 3)
        .unwrap();
    assert!(
        builder
            .modify_skill_points(Skill::from_str("Swordsmanship").unwrap(), -1)
            .is_ok()
    );
}
