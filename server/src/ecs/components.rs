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

//! ECS Components
//!
//! This module contains all component definitions for the game world.
//! Components are pure data structures that can be attached to entities.

mod ai;
pub mod character;
mod combat;
mod identity;
mod interaction;
mod npc;
mod persistence;
mod spatial;

// Re-export all components
pub use ai::*;
pub use character::*;
pub use combat::*;
pub use identity::*;
pub use interaction::*;
pub use npc::*;
pub use persistence::*;
pub use spatial::*;
