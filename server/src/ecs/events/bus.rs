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

//! Event bus implementation

use super::types::GameEvent;
use std::sync::{Arc, RwLock};

pub type EventHandler = Box<dyn Fn(&GameEvent) + Send + Sync>;

/// Event bus for publishing and subscribing to game events
pub struct EventBus {
    handlers: Arc<RwLock<Vec<EventHandler>>>,
    event_queue: Arc<RwLock<Vec<GameEvent>>>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(Vec::new())),
            event_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Subscribe to events with a handler function
    pub fn subscribe<F>(&self, handler: F)
    where
        F: Fn(&GameEvent) + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().unwrap();
        handlers.push(Box::new(handler));
    }

    /// Publish an event to the queue
    pub fn publish(&self, event: GameEvent) {
        let mut queue = self.event_queue.write().unwrap();
        queue.push(event);
    }

    /// Process all queued events
    pub fn process_events(&self) {
        let mut queue = self.event_queue.write().unwrap();
        let events: Vec<_> = queue.drain(..).collect();
        drop(queue);

        let handlers = self.handlers.read().unwrap();
        for event in events {
            for handler in handlers.iter() {
                handler(&event);
            }
        }
    }

    /// Clear all queued events without processing
    pub fn clear(&self) {
        let mut queue = self.event_queue.write().unwrap();
        queue.clear();
    }

    /// Get the number of queued events
    pub fn queue_len(&self) -> usize {
        let queue = self.event_queue.read().unwrap();
        queue.len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
            event_queue: Arc::clone(&self.event_queue),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_event_bus() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.subscribe(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(GameEvent::Custom {
            event_type: "test".into(),
            data: "data".into(),
        });

        assert_eq!(bus.queue_len(), 1);

        bus.process_events();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(bus.queue_len(), 0);
    }

    #[test]
    fn test_multiple_handlers() {
        let bus = EventBus::new();
        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));

        let c1 = Arc::clone(&counter1);
        let c2 = Arc::clone(&counter2);

        bus.subscribe(move |_| {
            c1.fetch_add(1, Ordering::SeqCst);
        });
        bus.subscribe(move |_| {
            c2.fetch_add(2, Ordering::SeqCst);
        });

        bus.publish(GameEvent::Custom {
            event_type: "test".into(),
            data: "data".into(),
        });

        bus.process_events();

        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_clear() {
        let bus = EventBus::new();

        bus.publish(GameEvent::Custom {
            event_type: "test".into(),
            data: "data".into(),
        });

        assert_eq!(bus.queue_len(), 1);
        bus.clear();
        assert_eq!(bus.queue_len(), 0);
    }
}
