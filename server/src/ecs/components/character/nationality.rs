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

/// Nationality of a character
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Nationality {
    /// Aurelian
    ///
    /// Nation: Aurelia
    /// Homeland: River valleys, marble cities, old roads
    /// Vibe: Disciplined elegance, inherited confidence
    /// Values: Duty, order, legacy
    /// Known for: Law, administration, professional armies
    ///
    /// Aurelians believe civilization is something you maintain. They take pride in institutions
    /// that outlast people—archives, codes, roads, academies. Even rebels tend to argue their
    /// case politely before starting a revolution.
    ///
    /// Stereotype: Arrogant but reliable
    /// Common phrase: “Stability is mercy.”
    Aurelian,

    /// Elyndric
    ///
    /// Nation: Elyndria
    /// Homeland: Forested realms woven with old magic
    /// Vibe: Polite, patient, slightly unsettling
    /// Values: Balance, memory, restraint
    /// Known for: Magical scholarship, diplomacy, ritual law
    ///
    /// Elyndric national identity is tied to continuity. They track ancestry, treaties, and
    /// grudges across centuries. They rarely rush—because they expect to still be around later.
    ///
    /// Stereotype: Manipulative traditionalists
    /// Common phrase: “Time will decide.”
    Elyndric,

    /// Kharuuni
    ///
    /// Nation: Kharuun
    /// Homeland: High plateaus and volcanic ridges
    /// Vibe: Stoic intensity, quiet pride
    /// Values: Endurance, honor, shared hardship
    /// Known for: Heavy infantry, stonecraft, oath-binding
    ///
    /// Kharuun don’t speak much, but when they promise something, it becomes part of their
    /// identity. Social status comes from what you’ve survived, not what you own.
    ///
    /// Stereotype: Grim and inflexible
    /// Common phrase: “The mountain remembers.”
    Kharuuni,

    /// Mandarian
    ///
    /// Nation: Mandaria
    /// Homeland: Cold frontiers, long winters, scattered holds
    /// Vibe: Blunt, resilient, dark humor
    /// Values: Mutual aid, honesty, practical skill
    /// Known for: Scouts, rangers, frontier engineers
    ///
    /// Mandarians also known as Northreachers distrust grand ideals but deeply trust neighbors.
    /// Their politics are simple: anyone who makes winter harder is an enemy.
    ///
    /// Stereotype: Crude but dependable
    /// Common phrase: “We get through it.”
    Mandarian,

    /// Sundari
    ///
    /// Nation: Sundar
    /// Homeland: Deserts, oasis-cities, sun-roads
    /// Vibe: Warm, expressive, sharp-eyed
    /// Values: Hospitality, cleverness, survival
    /// Known for: Caravan trade, oral history, subtle politics
    ///
    /// Among the Sundari, storytelling is a survival skill. Truth is respected—but presentation is
    /// everything. Outsiders are treated generously, but never naively.
    ///
    /// Stereotype: Charming schemers
    /// Common phrase: “Water shared is loyalty earned.”
    Sundari,

    /// Virethi
    ///
    /// Nation: Vireth
    /// Homeland: Windy coasts, island chains, storm-harbors
    /// Vibe: Restless, sharp-tongued, pragmatic
    /// Values: Freedom, reputation, adaptability
    /// Known for: Trade, navigation, mercenary contracts
    ///
    /// Virethi culture assumes that nothing is permanent—jobs, alliances, even names. Contracts
    /// matter more than bloodlines, and betrayal is less offensive than incompetence.
    ///
    /// Stereotype: Untrustworthy but effective
    /// Common phrase: “If it floats, it can be sold.”
    Virethi,
}

impl Nationality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Nationality::Aurelian => "Aurelian",
            Nationality::Elyndric => "Elyndric",
            Nationality::Kharuuni => "Kharuuni",
            Nationality::Mandarian => "Mandarian",
            Nationality::Sundari => "Sundari",
            Nationality::Virethi => "Virethi",
        }
    }
    pub fn nation(&self) -> &str {
        match self {
            Nationality::Aurelian => "Aurelia",
            Nationality::Elyndric => "Elyndria",
            Nationality::Kharuuni => "Kharuun",
            Nationality::Mandarian => "Mandaria",
            Nationality::Sundari => "Sundar",
            Nationality::Virethi => "Vireth",
        }
    }
}
