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

use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BodyAttributeScores(pub AttributeScores);

impl BodyAttributeScores {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MindAttributeScores(pub AttributeScores);

impl MindAttributeScores {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SoulAttributeScores(pub AttributeScores);

impl SoulAttributeScores {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Collection of scores for a single attribute class (Body, Mind, or Soul).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeScores {
    /// Offensive capability score.
    pub score_offence: i32,
    /// Finesse and precision score.
    pub score_finesse: i32,
    /// Defensive and stability score.
    pub score_defence: i32,
    /// Current health points.
    pub health_current: f32,
    /// Maximum health points.
    pub health_maximum: f32,
    /// Health regeneration per tick.
    pub health_regen: f32,
    /// Current energy points.
    pub energy_current: f32,
    /// Maximum energy points.
    pub energy_maximum: f32,
    /// Energy regeneration per tick.
    pub energy_regen: f32,
}

impl AttributeScores {
    /// Create a new default AttributeScores.
    pub fn new() -> Self {
        Self::default()
    }
    /// Create an Attribute Record from Database
    pub fn from_row(offense: i32, finesse: i32, defense: i32, health: f32, energy: f32) -> Self {
        let mut scores = Self {
            score_offence: offense,
            score_finesse: finesse,
            score_defence: defense,
            health_current: health,
            health_maximum: 0.0,
            health_regen: 0.0,
            energy_current: energy,
            energy_maximum: 0.0,
            energy_regen: 0.0,
        };
        scores.update_substats();
        scores
    }

    /// Recalculate maximum values and regeneration rates based on primary scores.
    pub fn update_substats(&mut self) {
        // Recalculate maximum and regen based on stats
        self.health_maximum = (self.score_offence + self.score_defence) as f32 * 10.0;
        self.health_regen = (self.score_finesse + self.score_defence) as f32 * 0.1;
        self.energy_maximum = (self.score_offence + self.score_finesse) as f32 * 10.0;
        self.energy_regen = (self.score_finesse + self.score_defence) as f32 * 0.1;
    }
}

impl Default for AttributeScores {
    fn default() -> Self {
        Self {
            score_offence: 10,
            score_finesse: 10,
            score_defence: 10,
            health_current: 100.0,
            health_maximum: 100.0,
            health_regen: 1.0,
            energy_current: 100.0,
            energy_maximum: 100.0,
            energy_regen: 1.0,
        }
    }
}

impl From<(i32, i32, i32, f32, f32)> for AttributeScores {
    fn from(value: (i32, i32, i32, f32, f32)) -> Self {
        AttributeScores::from_row(value.0, value.1, value.2, value.3, value.4)
    }
}

/// Primary categories of attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeClass {
    /// Physical aspect
    Body,
    /// Mental aspect
    Mind,
    /// Spiritual aspect
    Soul,
}

/// Specific attribute types for each class.
///
/// | Aspect    | Body      | Mind    | Soul       |
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
    /// Body - Power
    BodyOffence,
    /// Body - Control
    BodyFinesse,
    /// Body - Stability
    BodyDefence,

    /// Mind - Power
    MindOffence,
    /// Mind - Control
    MindFinesse,
    /// Mind - Stability
    MindDefence,

    /// Soul - Power
    SoulOffence,
    /// Soul - Control
    SoulFinesse,
    /// Soul - Stability
    SoulDefence,
}

impl AttributeType {
    /// Get the display name of the attribute.
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

    pub fn class(&self) -> AttributeClass {
        match self {
            AttributeType::BodyOffence => AttributeClass::Body,
            AttributeType::BodyFinesse => AttributeClass::Body,
            AttributeType::BodyDefence => AttributeClass::Body,
            AttributeType::MindOffence => AttributeClass::Mind,
            AttributeType::MindFinesse => AttributeClass::Mind,
            AttributeType::MindDefence => AttributeClass::Mind,
            AttributeType::SoulOffence => AttributeClass::Soul,
            AttributeType::SoulFinesse => AttributeClass::Soul,
            AttributeType::SoulDefence => AttributeClass::Soul,
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
            AttributeType::BodyOffence,
            AttributeType::BodyFinesse,
            AttributeType::BodyDefence,
            AttributeType::MindOffence,
            AttributeType::MindFinesse,
            AttributeType::MindDefence,
            AttributeType::SoulOffence,
            AttributeType::SoulFinesse,
            AttributeType::SoulDefence,
        ]
    }
}

impl FromStr for AttributeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bodyoffence" | "body_offence" | "bo" => Ok(AttributeType::BodyOffence),
            "bodyfinesse" | "body_finesse" | "bf" => Ok(AttributeType::BodyFinesse),
            "bodydefence" | "body_defence" | "bd" => Ok(AttributeType::BodyDefence),
            "mindoffence" | "mind_offence" | "mo" => Ok(AttributeType::MindOffence),
            "mindfinesse" | "mind_finesse" | "mf" => Ok(AttributeType::MindFinesse),
            "minddefence" | "mind_defence" | "md" => Ok(AttributeType::MindDefence),
            "souloffence" | "soul_offence" | "so" => Ok(AttributeType::SoulOffence),
            "soulfinesse" | "soul_finesse" | "sf" => Ok(AttributeType::SoulFinesse),
            "souldefence" | "soul_defence" | "sd" => Ok(AttributeType::SoulDefence),
            _ => Err(format!("Unknown attribute: {s}")),
        }
    }
}

/// Point costs for attribute ranks (progressive cost)
/// Rank 1-5: 1 point each
/// Rank 6-10: 2 points each
/// Rank 11-15: 3 points each
/// Rank 16-20: 4 points each
pub fn chargen_attribute_cost(rank: i32) -> i32 {
    match rank {
        1..=5 => 1,
        6..=10 => 2,
        11..=15 => 3,
        16..=20 => 4,
        _ => 0,
    }
}

/// Calculate total cost for an attribute at a given rank
pub fn chargen_total_attribute_cost(rank: i32) -> i32 {
    (1..=rank).map(chargen_attribute_cost).sum()
}
