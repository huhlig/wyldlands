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

use serde::{Deserialize, Serialize};

/// Body attributes (physical stats)
/// Maps to: entity_body_attributes table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyAttributes {
    pub score_offence: i32,
    pub score_finesse: i32,
    pub score_defence: i32,
    pub health_current: f32,
    pub health_maximum: f32,
    pub health_regen: f32,
    pub energy_current: f32,
    pub energy_maximum: f32,
    pub energy_regen: f32,
}

impl BodyAttributes {
    pub fn new() -> Self {
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

impl Default for BodyAttributes {
    fn default() -> Self {
        Self::new()
    }
}

/// Mind attributes (mental stats)
/// Maps to: entity_mind_attributes table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindAttributes {
    pub score_offence: i32,
    pub score_finesse: i32,
    pub score_defence: i32,
    pub health_current: f32,
    pub health_maximum: f32,
    pub health_regen: f32,
    pub energy_current: f32,
    pub energy_maximum: f32,
    pub energy_regen: f32,
}

impl MindAttributes {
    pub fn new() -> Self {
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

impl Default for MindAttributes {
    fn default() -> Self {
        Self::new()
    }
}

/// Soul attributes (spiritual stats)
/// Maps to: entity_soul_attributes table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulAttributes {
    pub score_offence: i32,
    pub score_finesse: i32,
    pub score_defence: i32,
    pub health_current: f32,
    pub health_maximum: f32,
    pub health_regen: f32,
    pub energy_current: f32,
    pub energy_maximum: f32,
    pub energy_regen: f32,
}

impl SoulAttributes {
    pub fn new() -> Self {
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

impl Default for SoulAttributes {
    fn default() -> Self {
        Self::new()
    }
}