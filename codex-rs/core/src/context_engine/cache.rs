//! Simple cache scaffolding for context engine.
use std::collections::HashMap;
use std::hash::Hash;

pub trait Cache<K, V> {
    fn get(&self, key: &K) -> Option<V>;
    fn insert(&mut self, key: K, value: V);
    fn clear(&mut self);
}

#[derive(Debug, Default)]
pub struct InMemoryCache<K, V> {
    map: HashMap<K, V>,
}

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> for InMemoryCache<K, V> {
    fn get(&self, key: &K) -> Option<V> {
        self.map.get(key).cloned()
    }
    fn insert(&mut self, key: K, value: V) {
        self.map.insert(key, value);
    }
    fn clear(&mut self) {
        self.map.clear();
    }
}
