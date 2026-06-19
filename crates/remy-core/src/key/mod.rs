pub use crate::keyboard::Keys;
pub mod handle;
pub use handle::{LiveKeys, StaticKeys};

use crate::runtime;

pub fn register(configure: impl FnOnce(&mut Keys)) -> runtime::LayerHandle {
    let mut keys = Keys::new();
    configure(&mut keys);
    runtime::add_layer(keys)
}

pub fn pending() -> Option<Vec<crate::keyboard::Key>> {
    runtime::pending_chord_keys()
}

pub fn pending_label() -> Option<String> {
    runtime::pending_chord_label()
}

pub fn completions() -> Vec<(crate::keyboard::Chord, String)> {
    runtime::chord_completions()
}