use std::cell::UnsafeCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use dashmap::DashMap;

use super::{SlotId, Value};

pub struct Slot {
    pub(super) current: UnsafeCell<Value>,
    pub(super) pending: UnsafeCell<Value>,
    pub(super) dirty: AtomicBool,
}

unsafe impl Send for Slot {}
unsafe impl Sync for Slot {}

impl Slot {
    fn new(initial: Value) -> Self {
        Self {
            current: UnsafeCell::new(Arc::clone(&initial)),
            pending: UnsafeCell::new(initial),
            dirty: AtomicBool::new(false),
        }
    }
}

pub struct Slots {
    slots: DashMap<SlotId, Slot>,
}

unsafe impl Send for Slots {}
unsafe impl Sync for Slots {}

impl Default for Slots {
    fn default() -> Self {
        Self::new()
    }
}

impl Slots {
    pub fn new() -> Self {
        Self {
            slots: DashMap::new(),
        }
    }

    pub fn allocate<T: Send + Sync + 'static>(&self, slot_id: SlotId, initial: T) {
        let arc: Value = Arc::new(initial);
        self.slots.insert(slot_id, Slot::new(arc));
    }

    pub fn read_current<T: 'static>(&self, slot_id: SlotId) -> &T {
        let entry = self
            .slots
            .get(&slot_id)
            .expect("slot read before allocation");
        let t_ptr: *const T = unsafe {
            let arc: &Value = &*entry.value().current.get();
            arc.downcast_ref::<T>().expect("type mismatch on slot read") as *const T
        };
        drop(entry);
        unsafe { &*t_ptr }
    }

    pub fn write_pending<T: Send + Sync + 'static>(&self, slot_id: SlotId, value: T) {
        if let Some(mut entry) = self.slots.get_mut(&slot_id) {
            let slot = entry.value_mut();
            unsafe {
                *slot.pending.get() = Arc::new(value);
            }
            slot.dirty.store(true, Ordering::Release);
        }
    }

    pub fn write_pending_raw(&self, slot_id: SlotId, value: Value) {
        if let Some(mut entry) = self.slots.get_mut(&slot_id) {
            let slot = entry.value_mut();
            unsafe {
                *slot.pending.get() = value;
            }
            slot.dirty.store(true, Ordering::Release);
        }
    }

    pub fn update_pending<T, F>(&self, slot_id: SlotId, f: F)
    where
        T: Send + Sync + Clone + 'static,
        F: FnOnce(&mut T),
    {
        if let Some(mut entry) = self.slots.get_mut(&slot_id) {
            let slot = entry.value_mut();
            let base = if slot.dirty.load(Ordering::Acquire) {
                unsafe { &*slot.pending.get() }
            } else {
                unsafe { &*slot.current.get() }
            };

            let mut value = base
                .downcast_ref::<T>()
                .expect("type mismatch on slot update")
                .clone();
            f(&mut value);

            unsafe {
                *slot.pending.get() = Arc::new(value);
            }
            slot.dirty.store(true, Ordering::Release);
        }
    }

    pub fn commit_all(&self) -> Vec<SlotId> {
        let mut committed = Vec::new();
        for mut entry in self.slots.iter_mut() {
            let slot_id = *entry.key();
            let slot = entry.value_mut();
            if slot.dirty.swap(false, Ordering::AcqRel) {
                unsafe {
                    std::ptr::swap(slot.current.get(), slot.pending.get());
                }
                committed.push(slot_id);
            }
        }
        committed
    }
}
