use std::cell::UnsafeCell;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    slots: RwLock<Vec<Option<Box<Slot>>>>,
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
            slots: RwLock::new(Vec::new()),
        }
    }

    fn slot_ptr(&self, slot_id: SlotId) -> *const Slot {
        let slots = self.slots.read().unwrap();
        let entry = slots
            .get(slot_id as usize)
            .and_then(|slot| slot.as_ref())
            .expect("slot read before allocation");
        &**entry as *const Slot
    }

    pub fn allocate<T: Send + Sync + 'static>(&self, slot_id: SlotId, initial: T) {
        let arc: Value = Arc::new(initial);
        let index = slot_id as usize;
        let mut slots = self.slots.write().unwrap();
        if index >= slots.len() {
            slots.resize_with(index + 1, || None);
        }
        slots[index] = Some(Box::new(Slot::new(arc)));
    }

    pub fn read_current<T: 'static>(&self, slot_id: SlotId) -> &T {
        let slot = self.slot_ptr(slot_id);
        unsafe {
            let arc: &Value = &*(*slot).current.get();
            arc.downcast_ref::<T>().expect("type mismatch on slot read")
        }
    }

    pub fn write_pending<T: Send + Sync + 'static>(&self, slot_id: SlotId, value: T) {
        let slot = self.slot_ptr(slot_id);
        unsafe {
            *(*slot).pending.get() = Arc::new(value);
            (*slot).dirty.store(true, Ordering::Release);
        }
    }

    pub fn write_pending_raw(&self, slot_id: SlotId, value: Value) {
        let slot = self.slot_ptr(slot_id);
        unsafe {
            *(*slot).pending.get() = value;
            (*slot).dirty.store(true, Ordering::Release);
        }
    }

    pub fn update_pending<T, F>(&self, slot_id: SlotId, f: F)
    where
        T: Send + Sync + Clone + 'static,
        F: FnOnce(&mut T),
    {
        let slot = self.slot_ptr(slot_id);
        unsafe {
            let base = if (*slot).dirty.load(Ordering::Acquire) {
                &*(*slot).pending.get()
            } else {
                &*(*slot).current.get()
            };
            let mut value = base
                .downcast_ref::<T>()
                .expect("type mismatch on slot update")
                .clone();
            f(&mut value);
            *(*slot).pending.get() = Arc::new(value);
            (*slot).dirty.store(true, Ordering::Release);
        }
    }

    pub fn commit_all(&self) -> Vec<SlotId> {
        let slots = self.slots.read().unwrap();
        let mut committed = Vec::new();
        for (index, slot) in slots.iter().enumerate() {
            let Some(slot) = slot else { continue };
            if slot.dirty.swap(false, Ordering::AcqRel) {
                unsafe {
                    std::ptr::swap(slot.current.get(), slot.pending.get());
                }
                committed.push(index as SlotId);
            }
        }
        committed
    }
}
