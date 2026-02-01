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

#[macro_export]
macro_rules! define_skills {
    (
        $(
            $id:ident {
                name: $name:expr,
                description: $description:expr,
                category: $category:expr,
                difficulty: $difficulty:expr,
                requires: $requires:expr,
                cost: $cost:expr,
            }
        ),* $(,)?
    ) => {
        /// Unique talent identifiers.
        #[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
        pub enum Skill {
            $(
                #[doc = $description]
                $id,
            )*
        }

        impl Skill {
            /// Get the name of this skill
            pub fn name(&self) -> &'static str {
                match self {
                    $(
                        Skill::$id => $name,
                    )*
                }
            }

            /// Get the description of this skill
            pub fn description(&self) -> &'static str {
                match self {
                    $(
                        Skill::$id => $description,
                    )*
                }
            }

            /// Get the category of this skill
            pub fn category(&self) -> SkillCategory {
                match self {
                    $(
                        Skill::$id => $category,
                    )*
                }
            }

            /// Get the difficulty of this skill
            pub fn difficulty(&self) -> SkillDifficulty {
                match self {
                    $(
                        Skill::$id => $difficulty,
                    )*
                }
            }

            /// Get the talent requirement for this skill, if any
            pub fn requires(&self) -> Option<Talent> {
                match self {
                    $(
                        Skill::$id => $requires,
                    )*
                }
            }

            /// Get the base cost of this skill at character Creation, None means it's not available
            pub fn cost(&self) -> Option<i32> {
                match self {
                    $(
                        Skill::$id => $cost,
                    )*
                }
            }

            /// Get all defined skills
            pub fn all() -> &'static [Skill] {
                &[
                    $(
                        Skill::$id,
                    )*
                ]
            }

            /// Get all available skills at character generation
            pub fn all_available(talents: &Talents) -> Vec<Skill> {
                Self::all().iter().filter(|&&skill| {
                    // Skills with cost = None are not available at character creation
                    if skill.cost().is_none() {
                        return false;
                    }
                    // Check talent requirements
                    if let Some(requires) = skill.requires() {
                        talents.has_talent(requires)
                    } else {
                        true
                    }
                }).copied().collect::<Vec<_>>()
            }
        }

        impl std::fmt::Display for Skill {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.name())
            }
        }

        impl FromStr for Skill {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::all().into_iter().find(|skill| skill.name().to_lowercase() == s.to_lowercase()).copied().ok_or(format!("Unknown Skill {}", s))
            }
        }
    };
}

#[macro_export]
macro_rules! define_talents {
    (
        $(
            $id:ident {
                name: $name:expr,
                description: $description:expr,
                category: $category:expr,
                requires: $requires:expr,
                cost: $cost:expr,
            }
        ),* $(,)?
    ) => {
        /// Unique talent identifiers.
        #[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
        pub enum Talent {
            $(
                #[doc = $description]
                $id,
            )*
        }

        impl Talent {
            /// Get the name of this talent
            pub fn name(&self) -> &'static str {
                match self {
                    $(
                        Talent::$id => $name,
                    )*
                }
            }

            /// Get the description of this talent
            pub fn description(&self) -> &'static str {
                match self {
                    $(
                        Talent::$id => $description,
                    )*
                }
            }

            /// Get the description of this talent
            pub fn category(&self) -> TalentCategory {
                match self {
                    $(
                        Talent::$id => $category,
                    )*
                }
            }

            /// Get the pre-requisite talent requirement for this talent, if any
            pub fn requires(&self) -> Option<Talent> {
                match self {
                    $(
                        Talent::$id => $requires,
                    )*
                }
            }

            /// Get the cost of this talent at character Creation, None means it's not available
            pub fn cost(&self) -> Option<i32> {
                match self {
                    $(
                        Talent::$id => $cost,
                    )*
                }
            }

            /// Get all available talents
            pub fn all() -> &'static [Talent] {
                &[
                    $(
                        Talent::$id,
                    )*
                ]
            }
        }

        impl std::fmt::Display for Talent {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.name())
            }
        }

        impl std::str::FromStr for Talent {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::all().into_iter().find(|skill| skill.name().to_lowercase() == s.to_lowercase()).copied().ok_or(format!("Unknown Talent {}", s))
            }
        }
    };
}
