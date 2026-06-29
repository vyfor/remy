use std::any::Any;
use std::sync::Arc;

mod id;
mod slots;

pub use id::next_slot_id;
pub use slots::{Slot, Slots};

pub type SlotId = u32;

pub(crate) type Value = Arc<dyn Any + Send + Sync>;
