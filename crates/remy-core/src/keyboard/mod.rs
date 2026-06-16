mod bind;
mod chord;
mod flow;
mod key;
mod keys;
mod mods;

pub use bind::{Bind, BindKind, ChordPolicy, IntoBind};
pub use chord::Chord;
pub use flow::{Flow, IntoFlow, quit};
pub use key::Key;
pub use keys::Keys;
pub use mods::Mods;

// maybe bump to 6?
pub const MAX_CHORD_LEN: usize = 4;
