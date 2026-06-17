use std::cell::{Cell, RefCell};
use std::collections::HashSet;

use crate::effect::EffectId;
use crate::state::SlotId;

mod effect;

pub use effect::{EffectGuard, current_effect, effect_context};

pub type OwnerId = u32;

thread_local! {
    pub(crate) static ACTIVE_EFFECT: Cell<Option<EffectId>> = const { Cell::new(None) };
    pub(crate) static ACTIVE_OWNER: Cell<Option<OwnerId>> = const { Cell::new(None) };
    pub(crate) static RENDER_TRACKING: Cell<bool> = const { Cell::new(false) };
    pub(crate) static RENDER_READS: RefCell<Vec<SlotId>> = const { RefCell::new(Vec::new()) };
    pub(crate) static OWNER_STACK: RefCell<Vec<Vec<SlotId>>> = const { RefCell::new(Vec::new()) };
    pub(crate) static DIRTY_SLOTS: RefCell<Vec<SlotId>> = const { RefCell::new(Vec::new()) };
}

pub fn begin_render_tracking() {
    RENDER_TRACKING.set(true);
}

pub fn end_render_tracking() {
    RENDER_TRACKING.set(false);
}

pub fn is_render_tracking() -> bool {
    RENDER_TRACKING.get()
}

pub fn record_render_read(slot_id: SlotId) {
    RENDER_READS.with(|reads| reads.borrow_mut().push(slot_id));
    OWNER_STACK.with(|stack| {
        if let Some(top) = stack.borrow_mut().last_mut() {
            top.push(slot_id);
        }
    });
}

pub fn drain_render_reads() -> Vec<SlotId> {
    RENDER_READS.with(|reads| std::mem::take(&mut *reads.borrow_mut()))
}

pub fn push_owner() {
    OWNER_STACK.with(|stack| stack.borrow_mut().push(Vec::new()));
}

pub fn pop_owner() -> Vec<SlotId> {
    let reads = OWNER_STACK
        .with(|stack| stack.borrow_mut().pop())
        .unwrap_or_default();
    OWNER_STACK.with(|stack| {
        if let Some(parent) = stack.borrow_mut().last_mut() {
            parent.extend(reads.iter().copied());
        }
    });
    reads
}

pub fn set_dirty_slots(slots: Vec<SlotId>) {
    DIRTY_SLOTS.with(|d| *d.borrow_mut() = slots);
}

pub fn any_slot_dirty(slots: &HashSet<SlotId>) -> bool {
    DIRTY_SLOTS.with(|d| d.borrow().iter().any(|s| slots.contains(s)))
}
