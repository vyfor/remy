use std::cell::{Cell, RefCell};

use crate::effect::EffectId;
use crate::state::SlotId;

mod effect;

pub use effect::{EffectGuard, current_effect, effect_context};

pub type OwnerId = u32;

thread_local! {
    pub(crate) static ACTIVE_EFFECT: Cell<Option<EffectId>> = const { Cell::new(None) };
    pub(crate) static ACTIVE_OWNER: Cell<Option<OwnerId>> = const { Cell::new(None) };
    pub(crate) static RENDER_TRACKING: Cell<bool> = const { Cell::new(false) };
    // unfortunate
    pub(crate) static RENDER_READS: RefCell<Vec<SlotId>> = const { RefCell::new(Vec::new()) };
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
}

pub fn drain_render_reads() -> Vec<SlotId> {
    RENDER_READS.with(|reads| std::mem::take(&mut *reads.borrow_mut()))
}
