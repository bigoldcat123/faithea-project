use std::collections::HashMap;

pub struct Trie<T, K> {
    pub children: HashMap<K, Box<Self>>,
    pub value: T,
}

impl<T: Default, K> Default for Trie<T, K> {
    fn default() -> Self {
        Self {
            children: Default::default(),
            value: Default::default(),
        }
    }
}
