use std::sync::atomic::{AtomicU32, Ordering};

use super::SlotId;

static NEXT_RUNTIME_SLOT_ID: AtomicU32 = AtomicU32::new(0);

pub fn next_slot_id() -> SlotId {
    NEXT_RUNTIME_SLOT_ID.fetch_add(1, Ordering::Relaxed)
}

pub const fn const_slot_id(module_path: &str, var_name: &str) -> SlotId {
    let mut hash: u32 = 2166136261;
    let module_bytes = module_path.as_bytes();
    let var_bytes = var_name.as_bytes();

    let mut i = 0;
    while i < module_bytes.len() {
        hash ^= module_bytes[i] as u32;
        hash = hash.wrapping_mul(16777619);
        i += 1;
    }

    hash ^= b':' as u32;
    hash = hash.wrapping_mul(16777619);

    i = 0;
    while i < var_bytes.len() {
        hash ^= var_bytes[i] as u32;
        hash = hash.wrapping_mul(16777619);
        i += 1;
    }

    hash
}
