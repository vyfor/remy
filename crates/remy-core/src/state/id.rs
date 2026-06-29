use std::sync::atomic::{AtomicU32, Ordering};

use super::SlotId;

static NEXT_RUNTIME_SLOT_ID: AtomicU32 = AtomicU32::new(0);

pub fn next_slot_id() -> SlotId {
    NEXT_RUNTIME_SLOT_ID.fetch_add(1, Ordering::Relaxed)
}
