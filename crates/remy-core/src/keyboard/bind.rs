use super::{Chord, Key};

pub trait IntoBind {
    fn into_key_binding(self) -> Chord;
}

impl IntoBind for char {
    fn into_key_binding(self) -> Chord {
        Chord::from(self)
    }
}

impl IntoBind for Key {
    fn into_key_binding(self) -> Chord {
        Chord::from(self)
    }
}

impl IntoBind for Chord {
    fn into_key_binding(self) -> Chord {
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChordPolicy {
    #[default]
    Discard,
    Fallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindKind {
    Single,
    Chord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bind {
    pub keys: Chord,
    pub label: String,
    pub kind: BindKind,
}
