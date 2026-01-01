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

//! Combat components for fighting and equipment

use super::EntityId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(test)]
use uuid::Uuid;

/// Status effect types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusEffectType {
    Stunned,
    Poisoned,
    Burning,
    Bleeding,
    Defending,
    Weakened,
    Strengthened,
    Slowed,
    Hasted,
}

impl StatusEffectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusEffectType::Stunned => "Stunned",
            StatusEffectType::Poisoned => "Poisoned",
            StatusEffectType::Burning => "Burning",
            StatusEffectType::Bleeding => "Bleeding",
            StatusEffectType::Defending => "Defending",
            StatusEffectType::Weakened => "Weakened",
            StatusEffectType::Strengthened => "Strengthened",
            StatusEffectType::Slowed => "Slowed",
            StatusEffectType::Hasted => "Hasted",
        }
    }
}

/// Individual status effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffect {
    pub effect_type: StatusEffectType,
    pub duration: f32,
    pub magnitude: i32,
}

impl StatusEffect {
    pub fn new(effect_type: StatusEffectType, duration: f32, magnitude: i32) -> Self {
        Self {
            effect_type,
            duration,
            magnitude,
        }
    }
}

/// Status effects component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffects {
    pub effects: Vec<StatusEffect>,
}

impl StatusEffects {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    pub fn add_effect(&mut self, effect: StatusEffect) {
        // Remove existing effect of same type
        self.effects.retain(|e| e.effect_type != effect.effect_type);
        self.effects.push(effect);
    }

    pub fn remove_effect(&mut self, effect_type: StatusEffectType) {
        self.effects.retain(|e| e.effect_type != effect_type);
    }

    pub fn has_effect(&self, effect_type: StatusEffectType) -> bool {
        self.effects.iter().any(|e| e.effect_type == effect_type)
    }

    pub fn get_effect(&self, effect_type: StatusEffectType) -> Option<&StatusEffect> {
        self.effects.iter().find(|e| e.effect_type == effect_type)
    }

    pub fn update(&mut self, delta_time: f32) {
        // Update durations and remove expired effects
        for effect in &mut self.effects {
            effect.duration -= delta_time;
        }
        self.effects.retain(|e| e.duration > 0.0);
    }
}

impl Default for StatusEffects {
    fn default() -> Self {
        Self::new()
    }
}

/// Combat state component
/// Maps to: entity_combatant table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combatant {
    pub in_combat: bool,
    pub target_id: Option<EntityId>,
    pub initiative: i32,
    pub action_cooldown: f32,
    pub time_since_action: f32,
    pub is_defending: bool,
    pub defense_bonus: i32,
}

impl Combatant {
    /// Create a new combatant
    pub fn new() -> Self {
        Self {
            in_combat: false,
            target_id: None,
            initiative: 0,
            action_cooldown: 1.0,
            time_since_action: 1.0,
            is_defending: false,
            defense_bonus: 0,
        }
    }
    
    /// Check if the combatant can attack
    pub fn can_attack(&self) -> bool {
        self.time_since_action >= self.action_cooldown
    }
    
    /// Update attack timer
    pub fn update_timer(&mut self, delta_time: f32) {
        self.time_since_action += delta_time;
    }
    
    /// Reset attack timer
    pub fn reset_timer(&mut self) {
        self.time_since_action = 0.0;
    }

    /// Start defending
    pub fn start_defending(&mut self, bonus: i32) {
        self.is_defending = true;
        self.defense_bonus = bonus;
    }

    /// Stop defending
    pub fn stop_defending(&mut self) {
        self.is_defending = false;
        self.defense_bonus = 0;
    }
}

impl Default for Combatant {
    fn default() -> Self {
        Self::new()
    }
}

/// Equipment slots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    Head,
    Chest,
    Legs,
    Feet,
    Hands,
    MainHand,
    OffHand,
    Ring1,
    Ring2,
    Neck,
    Back,
    Tail,
    Wings,
}

impl EquipSlot {
    pub fn as_str(&self) -> &'static str {
        match self {
            EquipSlot::Head => "Head",
            EquipSlot::Chest => "Chest",
            EquipSlot::Legs => "Legs",
            EquipSlot::Feet => "Feet",
            EquipSlot::Hands => "Hands",
            EquipSlot::MainHand => "MainHand",
            EquipSlot::OffHand => "OffHand",
            EquipSlot::Ring1 => "Ring1",
            EquipSlot::Ring2 => "Ring2",
            EquipSlot::Neck => "Neck",
            EquipSlot::Back => "Back",
            EquipSlot::Tail => "Tail",
            EquipSlot::Wings => "Wings",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Head" => Some(EquipSlot::Head),
            "Chest" => Some(EquipSlot::Chest),
            "Legs" => Some(EquipSlot::Legs),
            "Feet" => Some(EquipSlot::Feet),
            "Hands" => Some(EquipSlot::Hands),
            "MainHand" => Some(EquipSlot::MainHand),
            "OffHand" => Some(EquipSlot::OffHand),
            "Ring1" => Some(EquipSlot::Ring1),
            "Ring2" => Some(EquipSlot::Ring2),
            "Neck" => Some(EquipSlot::Neck),
            "Back" => Some(EquipSlot::Back),
            "Tail" => Some(EquipSlot::Tail),
            "Wings" => Some(EquipSlot::Wings),
            _ => None,
        }
    }
}

/// Equipment slots and worn items
/// Maps to: entity_equipment table (one row per slot)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equipment {
    pub slots: HashMap<EquipSlot, EntityId>,
}

impl Equipment {
    /// Create new empty equipment
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    /// Equip an item in a slot, returns previously equipped item if any
    pub fn equip(&mut self, slot: EquipSlot, item: EntityId) -> Option<EntityId> {
        self.slots.insert(slot, item)
    }

    /// Unequip an item from a slot
    pub fn unequip(&mut self, slot: EquipSlot) -> Option<EntityId> {
        self.slots.remove(&slot)
    }

    /// Get the item in a slot
    pub fn get(&self, slot: EquipSlot) -> Option<EntityId> {
        self.slots.get(&slot).copied()
    }
    
    /// Check if a slot is occupied
    pub fn is_equipped(&self, slot: EquipSlot) -> bool {
        self.slots.contains_key(&slot)
    }
}

impl Default for Equipment {
    fn default() -> Self {
        Self::new()
    }
}

/// Equipable component - marks items that can be equipped
/// Maps to: entity_equipable table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equipable {
    pub slots: Vec<EquipSlot>,
}

impl Equipable {
    pub fn new(slots: Vec<EquipSlot>) -> Self {
        Self { slots }
    }
}

/// Damage types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageType {
    Blunt,
    Piercing,
    Slashing,
    Fire,
    Acid,
    Arcane,
    Psychic,
}

impl DamageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DamageType::Blunt => "Blunt",
            DamageType::Piercing => "Piercing",
            DamageType::Slashing => "Slashing",
            DamageType::Fire => "Fire",
            DamageType::Acid => "Acid",
            DamageType::Arcane => "Arcane",
            DamageType::Psychic => "Psychic",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Blunt" => Some(DamageType::Blunt),
            "Piercing" => Some(DamageType::Piercing),
            "Slashing" => Some(DamageType::Slashing),
            "Fire" => Some(DamageType::Fire),
            "Acid" => Some(DamageType::Acid),
            "Arcane" => Some(DamageType::Arcane),
            "Psychic" => Some(DamageType::Psychic),
            _ => None,
        }
    }
}

/// Weapon properties
/// Maps to: entity_weapon table
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Weapon {
    pub damage_min: i32,
    pub damage_max: i32,
    pub damage_cap: i32,
    pub damage_type: DamageType,
    pub attack_speed: f32,
    pub range: f32,
}

impl Weapon {
    /// Create a new weapon
    pub fn new(damage_min: i32, damage_max: i32, damage_type: DamageType) -> Self {
        Self {
            damage_min,
            damage_max,
            damage_cap: damage_max,
            damage_type,
            attack_speed: 1.0,
            range: 1.0,
        }
    }
}

/// Material types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialKind {
    Cloth,
    Leather,
    Chain,
    Iron,
    Steel,
    Mana,
}

impl MaterialKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaterialKind::Cloth => "Cloth",
            MaterialKind::Leather => "Leather",
            MaterialKind::Chain => "Chain",
            MaterialKind::Iron => "Iron",
            MaterialKind::Steel => "Steel",
            MaterialKind::Mana => "Mana",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Cloth" => Some(MaterialKind::Cloth),
            "Leather" => Some(MaterialKind::Leather),
            "Chain" => Some(MaterialKind::Chain),
            "Iron" => Some(MaterialKind::Iron),
            "Steel" => Some(MaterialKind::Steel),
            "Mana" => Some(MaterialKind::Mana),
            _ => None,
        }
    }
}

/// Material component
/// Maps to: entity_material table
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Material {
    pub material_kind: MaterialKind,
}

impl Material {
    pub fn new(material_kind: MaterialKind) -> Self {
        Self { material_kind }
    }
}

/// Armor defense component
/// Maps to: entity_armor_defense table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmorDefense {
    pub defenses: HashMap<DamageType, i32>,
}

impl ArmorDefense {
    pub fn new() -> Self {
        Self {
            defenses: HashMap::new(),
        }
    }
    
    pub fn set_defense(&mut self, damage_kind: DamageType, defense: i32) {
        self.defenses.insert(damage_kind, defense);
    }
    
    pub fn get_defense(&self, damage_kind: DamageType) -> i32 {
        self.defenses.get(&damage_kind).copied().unwrap_or(0)
    }
}

impl Default for ArmorDefense {
    fn default() -> Self {
        Self::new()
    }
}

// Legacy compatibility types (deprecated)
#[deprecated(note = "Use DamageType instead")]
pub type WeaponType = DamageType;

#[deprecated(note = "Use MaterialKind instead")]
pub type ArmorType = MaterialKind;

#[deprecated(note = "Use ArmorDefense instead")]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Armor {
    pub defense: i32,
    pub armor_type: MaterialKind,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_combatant_attack_cooldown() {
        let mut combatant = Combatant::new();
        assert!(combatant.can_attack());
        
        combatant.reset_timer();
        assert!(!combatant.can_attack());
        
        combatant.update_timer(1.0);
        assert!(combatant.can_attack());
    }
    
    #[test]
    fn test_equipment() {
        let mut equipment = Equipment::new();
        let sword = EntityId::from_uuid(Uuid::new_v4());
        let shield = EntityId::from_uuid(Uuid::new_v4());

        assert!(equipment.equip(EquipSlot::MainHand, sword).is_none());
        assert!(equipment.is_equipped(EquipSlot::MainHand));
        assert_eq!(equipment.get(EquipSlot::MainHand), Some(sword));

        equipment.equip(EquipSlot::OffHand, shield);
        assert_eq!(equipment.slots.len(), 2);
        
        let old_sword = equipment.equip(EquipSlot::MainHand, shield);
        assert_eq!(old_sword, Some(sword));
    }
    
    #[test]
    fn test_weapon() {
        let weapon = Weapon::new(5, 10, DamageType::Slashing);
        assert_eq!(weapon.damage_min, 5);
        assert_eq!(weapon.damage_max, 10);
        assert_eq!(weapon.damage_type, DamageType::Slashing);
    }
    
    #[test]
    fn test_armor_defense() {
        let mut armor = ArmorDefense::new();
        armor.set_defense(DamageType::Slashing, 10);
        armor.set_defense(DamageType::Blunt, 5);
        
        assert_eq!(armor.get_defense(DamageType::Slashing), 10);
        assert_eq!(armor.get_defense(DamageType::Fire), 0);
    }
}


