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

//! Interactive character sheet display for telnet

use wyldlands_common::character::{attribute_cost, skill_cost, AttributeType, CharacterBuilder, Talent};

/// Format the character sheet for display
pub fn format_character_sheet(builder: &CharacterBuilder) -> String {
    let mut output = String::new();

    output.push_str("\r\n");
    output.push_str(
        "╔════════════════════════════════════════════════════════════════════════════╗\r\n",
    );
    output.push_str(&format!("║ CHARACTER CREATION: {:<58} ║\r\n", builder.name));
    output.push_str(
        "╠════════════════════════════════════════════════════════════════════════════╣\r\n",
    );

    // Point pools
    output.push_str(&format!(
        "║ Attribute/Talent Points: {}/{:<3}  Skill Points: {}/{:<3}                  ║\r\n",
        builder.attribute_talent_points,
        builder.max_attribute_talent_points,
        builder.skill_points,
        builder.max_skill_points
    ));
    output.push_str(
        "╠════════════════════════════════════════════════════════════════════════════╣\r\n",
    );

    // Attributes section
    output.push_str(
        "║ ATTRIBUTES                                                                 ║\r\n",
    );
    output.push_str(
        "╟────────────────────────────────────────────────────────────────────────────╢\r\n",
    );

    // Body attributes
    output.push_str(
        "║ Body:                                                                      ║\r\n",
    );
    for attr in [
        AttributeType::BodyOffence,
        AttributeType::BodyFinesse,
        AttributeType::BodyDefence,
    ] {
        let rank = builder.get_attribute(attr);
        let next_cost = if rank < 20 {
            attribute_cost(rank + 1)
        } else {
            0
        };
        output.push_str(&format!(
            "║   {:15} [{:2}]  +[A]  -[B]  (Next: {} pts)                       ║\r\n",
            attr.name(),
            rank,
            if next_cost > 0 {
                next_cost.to_string()
            } else {
                "-".to_string()
            }
        ));
    }

    // Mind attributes
    output.push_str(
        "║ Mind:                                                                      ║\r\n",
    );
    for attr in [
        AttributeType::MindOffence,
        AttributeType::MindFinesse,
        AttributeType::MindDefence,
    ] {
        let rank = builder.get_attribute(attr);
        let next_cost = if rank < 20 {
            attribute_cost(rank + 1)
        } else {
            0
        };
        output.push_str(&format!(
            "║   {:15} [{:2}]  +[C]  -[D]  (Next: {} pts)                       ║\r\n",
            attr.name(),
            rank,
            if next_cost > 0 {
                next_cost.to_string()
            } else {
                "-".to_string()
            }
        ));
    }

    // Soul attributes
    output.push_str(
        "║ Soul:                                                                      ║\r\n",
    );
    for attr in [
        AttributeType::SoulOffence,
        AttributeType::SoulFinesse,
        AttributeType::SoulDefence,
    ] {
        let rank = builder.get_attribute(attr);
        let next_cost = if rank < 20 {
            attribute_cost(rank + 1)
        } else {
            0
        };
        output.push_str(&format!(
            "║   {:15} [{:2}]  +[E]  -[F]  (Next: {} pts)                       ║\r\n",
            attr.name(),
            rank,
            if next_cost > 0 {
                next_cost.to_string()
            } else {
                "-".to_string()
            }
        ));
    }

    output.push_str(
        "╟────────────────────────────────────────────────────────────────────────────╢\r\n",
    );

    // Talents section
    output.push_str(
        "║ TALENTS                                                                    ║\r\n",
    );
    output.push_str(
        "╟────────────────────────────────────────────────────────────────────────────╢\r\n",
    );

    if builder.talents.is_empty() {
        output.push_str(
            "║   No talents selected. Type 'talents' to view available talents.          ║\r\n",
        );
    } else {
        for talent in &builder.talents {
            output.push_str(&format!(
                "║   {:20} ({} pts) - Remove: talent remove {}              ║\r\n",
                talent.name(),
                talent.cost(),
                talent.name().to_lowercase().replace(" ", "_")
            ));
        }
    }

    output.push_str(
        "╟────────────────────────────────────────────────────────────────────────────╢\r\n",
    );

    // Skills section
    output.push_str(
        "║ SKILLS                                                                     ║\r\n",
    );
    output.push_str(
        "╟────────────────────────────────────────────────────────────────────────────╢\r\n",
    );

    if builder.skills.is_empty() {
        output.push_str(
            "║   No skills selected. Type 'skills' to view available skills.             ║\r\n",
        );
    } else {
        let mut skills: Vec<_> = builder.skills.iter().collect();
        skills.sort_by_key(|(name, _)| *name);

        for (skill_name, &rank) in skills {
            let next_cost = if rank < 10 {
                skill_cost(rank + 1)
            } else {
                0
            };
            output.push_str(&format!(
                "║   {:20} [{:2}]  (Next: {} pts)                              ║\r\n",
                skill_name,
                rank,
                if next_cost > 0 {
                    next_cost.to_string()
                } else {
                    "-".to_string()
                }
            ));
        }
    }

    output.push_str(
        "╠════════════════════════════════════════════════════════════════════════════╣\r\n",
    );
    output.push_str(
        "║ COMMANDS:                                                                  ║\r\n",
    );
    output.push_str(
        "║   attr <body|mind|soul> <off|fin|def> <+|->  - Adjust attributes           ║\r\n",
    );
    output.push_str(
        "║   talents                                     - View available talents     ║\r\n",
    );
    output.push_str(
        "║   talent add <name>                           - Add a talent               ║\r\n",
    );
    output.push_str(
        "║   talent remove <name>                        - Remove a talent            ║\r\n",
    );
    output.push_str(
        "║   skills                                      - View available skills      ║\r\n",
    );
    output.push_str(
        "║   skill <name> <+|->                          - Adjust skill               ║\r\n",
    );
    output.push_str(
        "║   done                                        - Finish character creation  ║\r\n",
    );
    output.push_str(
        "║   cancel                                      - Cancel character creation  ║\r\n",
    );
    output.push_str(
        "╚════════════════════════════════════════════════════════════════════════════╝\r\n",
    );
    output.push_str("\r\n> ");

    output
}

/// Format the talents list for display
pub fn format_talents_list() -> String {
    let mut output = String::new();

    output.push_str("\r\n");
    output.push_str(
        "╔════════════════════════════════════════════════════════════════════════════╗\r\n",
    );
    output.push_str(
        "║ AVAILABLE TALENTS                                                          ║\r\n",
    );
    output.push_str(
        "╠════════════════════════════════════════════════════════════════════════════╣\r\n",
    );

    // Group talents by cost
    let mut talents_by_cost: std::collections::HashMap<i32, Vec<Talent>> =
        std::collections::HashMap::new();
    for talent in Talent::all() {
        talents_by_cost
            .entry(talent.cost())
            .or_insert_with(Vec::new)
            .push(talent);
    }

    let mut costs: Vec<_> = talents_by_cost.keys().copied().collect();
    costs.sort();

    for cost in costs {
        output.push_str(&format!(
            "║ {} Point Talents:                                                           ║\r\n",
            cost
        ));
        output.push_str(
            "╟────────────────────────────────────────────────────────────────────────────╢\r\n",
        );

        if let Some(talents) = talents_by_cost.get(&cost) {
            for talent in talents {
                output.push_str(&format!(
                    "║ {:20} - {:<50} ║\r\n",
                    talent.name(),
                    talent.description()
                ));
                output.push_str(&format!(
                    "║   Add: talent add {:<58} ║\r\n",
                    talent.name().to_lowercase().replace(" ", "_")
                ));
            }
        }
        output.push_str(
            "╟────────────────────────────────────────────────────────────────────────────╢\r\n",
        );
    }

    output.push_str(
        "║ Type 'sheet' to return to character sheet                                  ║\r\n",
    );
    output.push_str(
        "╚════════════════════════════════════════════════════════════════════════════╝\r\n",
    );
    output.push_str("\r\n> ");

    output
}

/// Format the skills list for display
pub fn format_skills_list() -> String {
    let mut output = String::new();

    output.push_str("\r\n");
    output.push_str(
        "╔════════════════════════════════════════════════════════════════════════════╗\r\n",
    );
    output.push_str(
        "║ AVAILABLE SKILLS                                                           ║\r\n",
    );
    output.push_str(
        "╠════════════════════════════════════════════════════════════════════════════╣\r\n",
    );
    output.push_str(
        "║ Common Skills:                                                             ║\r\n",
    );
    output.push_str(
        "║   Swordsmanship, Archery, Stealth, Persuasion, Tracking, Herbalism         ║\r\n",
    );
    output.push_str(
        "║   Alchemy, Blacksmithing, Cooking, First Aid, Navigation                   ║\r\n",
    );
    output.push_str(
        "╟────────────────────────────────────────────────────────────────────────────╢\r\n",
    );
    output.push_str(
        "║ Magic Skills:                                                              ║\r\n",
    );
    output.push_str(
        "║   Evocation, Conjuration, Illusion, Enchantment, Divination                ║\r\n",
    );
    output.push_str(
        "║   Necromancy, Transmutation, Abjuration                                    ║\r\n",
    );
    output.push_str(
        "╟────────────────────────────────────────────────────────────────────────────╢\r\n",
    );
    output.push_str(
        "║ Skill Costs:                                                               ║\r\n",
    );
    output.push_str(
        "║   Ranks 1-3:  1 point each                                                 ║\r\n",
    );
    output.push_str(
        "║   Ranks 4-6:  2 points each                                                ║\r\n",
    );
    output.push_str(
        "║   Ranks 7-9:  3 points each                                                ║\r\n",
    );
    output.push_str(
        "║   Rank 10:    4 points                                                     ║\r\n",
    );
    output.push_str(
        "╟────────────────────────────────────────────────────────────────────────────╢\r\n",
    );
    output.push_str(
        "║ Usage: skill <name> +     - Increase skill by 1 rank                       ║\r\n",
    );
    output.push_str(
        "║        skill <name> -     - Decrease skill by 1 rank                       ║\r\n",
    );
    output.push_str(
        "║ Type 'sheet' to return to character sheet                                  ║\r\n",
    );
    output.push_str(
        "╚════════════════════════════════════════════════════════════════════════════╝\r\n",
    );
    output.push_str("\r\n> ");

    output
}

/// Parse attribute command (e.g., "attr body off +")
pub fn parse_attribute_command(input: &str) -> Option<(AttributeType, bool)> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 4 || parts[0] != "attr" {
        return None;
    }

    let attr_type = match (parts[1], parts[2]) {
        ("body", "off") | ("body", "offence") => AttributeType::BodyOffence,
        ("body", "fin") | ("body", "finesse") => AttributeType::BodyFinesse,
        ("body", "def") | ("body", "defence") | ("body", "defense") => AttributeType::BodyDefence,
        ("mind", "off") | ("mind", "offence") => AttributeType::MindOffence,
        ("mind", "fin") | ("mind", "finesse") => AttributeType::MindFinesse,
        ("mind", "def") | ("mind", "defence") | ("mind", "defense") => AttributeType::MindDefence,
        ("soul", "off") | ("soul", "offence") => AttributeType::SoulOffence,
        ("soul", "fin") | ("soul", "finesse") => AttributeType::SoulFinesse,
        ("soul", "def") | ("soul", "defence") | ("soul", "defense") => AttributeType::SoulDefence,
        _ => return None,
    };

    let increase = match parts[3] {
        "+" | "inc" | "increase" => true,
        "-" | "dec" | "decrease" => false,
        _ => return None,
    };

    Some((attr_type, increase))
}

/// Parse skill command (e.g., "skill swordsmanship +")
pub fn parse_skill_command(input: &str) -> Option<(String, bool)> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 3 || parts[0] != "skill" {
        return None;
    }

    let skill_name = parts[1].to_string();
    let increase = match parts[2] {
        "+" | "inc" | "increase" => true,
        "-" | "dec" | "decrease" => false,
        _ => return None,
    };

    Some((skill_name, increase))
}

/// Parse talent command (e.g., "talent add weapon_master")
pub fn parse_talent_command(input: &str) -> Option<(bool, String)> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 3 || parts[0] != "talent" {
        return None;
    }

    let add = match parts[1] {
        "add" => true,
        "remove" | "rem" => false,
        _ => return None,
    };

    let talent_name = parts[2].replace("_", " ");
    Some((add, talent_name))
}

/// Find talent by name (case-insensitive)
pub fn find_talent_by_name(name: &str) -> Option<Talent> {
    let name_lower = name.to_lowercase();
    Talent::all()
        .into_iter()
        .find(|t| t.name().to_lowercase() == name_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_attribute_command() {
        assert_eq!(
            parse_attribute_command("attr body off +"),
            Some((AttributeType::BodyOffence, true))
        );
        assert_eq!(
            parse_attribute_command("attr mind def -"),
            Some((AttributeType::MindDefence, false))
        );
        assert_eq!(parse_attribute_command("invalid"), None);
    }

    #[test]
    fn test_parse_skill_command() {
        assert_eq!(
            parse_skill_command("skill swordsmanship +"),
            Some(("swordsmanship".to_string(), true))
        );
        assert_eq!(
            parse_skill_command("skill archery -"),
            Some(("archery".to_string(), false))
        );
    }

    #[test]
    fn test_parse_talent_command() {
        assert_eq!(
            parse_talent_command("talent add weapon_master"),
            Some((true, "weapon master".to_string()))
        );
        assert_eq!(
            parse_talent_command("talent remove lucky"),
            Some((false, "lucky".to_string()))
        );
    }
}


