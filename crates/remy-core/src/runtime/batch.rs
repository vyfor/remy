use std::cell::{Cell, RefCell};

use crate::effect::EffectId;

use super::{Runtime, apply_commits};

thread_local! {
    static BATCH_DEPTH: Cell<u32> = const { Cell::new(0) };
    pub(crate) static BATCH_QUEUE: RefCell<Vec<EffectId>> = const { RefCell::new(Vec::new()) };
}

pub fn is_batching() -> bool {
    BATCH_DEPTH.with(|d| d.get()) > 0
}

pub fn batch_enter() {
    BATCH_DEPTH.with(|d| d.set(d.get() + 1));
}

pub fn batch_exit() {
    let prev = BATCH_DEPTH.with(|d| d.get());
    if prev == 1 {
        apply_commits();
    }
    BATCH_DEPTH.with(|d| d.set(prev - 1));
    if prev == 1 {
        flush_batch();
    }
}

pub fn flush_batch() {
    loop {
        let is_empty = BATCH_QUEUE.with(|q| q.borrow().is_empty());
        if is_empty {
            return;
        }
        let ids: Vec<EffectId> = BATCH_QUEUE.with(|q| {
            let mut queue = q.borrow_mut();
            queue.sort_unstable();
            queue.dedup();
            std::mem::take(&mut *queue)
        });
        if ids.is_empty() {
            return;
        }
        Runtime::get().effects.run_ids(&ids);
    }
}
