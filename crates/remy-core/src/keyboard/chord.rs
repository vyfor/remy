use smallvec::SmallVec;

use super::{Key, MAX_CHORD_LEN};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Chord {
    keys: SmallVec<[Key; MAX_CHORD_LEN]>,
}

impl Chord {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_keys<I, K>(keys: I) -> Self
    where
        I: IntoIterator<Item = K>,
        K: Into<Key>,
    {
        let mut chord = Self::new();
        for key in keys {
            chord.push(key.into());
        }
        chord
    }

    pub fn then(mut self, next: impl Into<Key>) -> Self {
        self.push(next.into());
        self
    }

    pub fn label(&self) -> String {
        self.keys
            .iter()
            .map(Key::label)
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn as_slice(&self) -> &[Key] {
        &self.keys
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn first(&self) -> Option<Key> {
        self.keys.first().copied()
    }

    pub fn starts_with(&self, prefix: &Chord) -> bool {
        self.keys.starts_with(prefix.as_slice())
    }

    fn push(&mut self, key: Key) {
        debug_assert!(
            self.keys.len() < MAX_CHORD_LEN,
            "key chord shouldnt exceed {MAX_CHORD_LEN} keys",
        );
        self.keys.push(key);
    }

    pub(crate) fn prefix(&self, len: usize) -> Self {
        let mut keys = SmallVec::new();
        keys.extend_from_slice(&self.keys[..len]);
        Self { keys }
    }
}

impl From<Key> for Chord {
    fn from(key: Key) -> Self {
        Self::from_keys([key])
    }
}

impl From<char> for Chord {
    fn from(c: char) -> Self {
        Self::from(Key::from(c))
    }
}
