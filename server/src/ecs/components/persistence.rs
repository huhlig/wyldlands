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

//! Persistence components for database synchronization

use serde::{Deserialize, Serialize};

/// Marks entities that should be persisted to the database
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Persistent;

/// Marks entities that have been modified and need to be saved
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dirty;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistence_markers() {
        let _persistent = Persistent;
        let _dirty = Dirty;
        // These are just marker components, no behavior to test
    }
}
