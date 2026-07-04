use std::cell::UnsafeCell;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

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

const FIRST_BITS: u32 = 5;
const FIRST_LEN: u64 = 1 << FIRST_BITS;
const BUCKETS: usize = 32;

fn location(slot_id: SlotId) -> (usize, usize) {
    let pos = slot_id as u64 + FIRST_LEN;
    let level = 63 - pos.leading_zeros() as usize;
    let bucket = level - FIRST_BITS as usize;
    let offset = (pos - (1 << level)) as usize;
    (bucket, offset)
}

fn bucket_len(bucket: usize) -> usize {
    1 << (bucket + FIRST_BITS as usize)
}

pub struct Slots {
    buckets: [AtomicPtr<AtomicPtr<Slot>>; BUCKETS],
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
            buckets: std::array::from_fn(|_| AtomicPtr::new(ptr::null_mut())),
        }
    }

    fn ensure_bucket(&self, bucket: usize) -> *mut AtomicPtr<Slot> {
        let existing = self.buckets[bucket].load(Ordering::Acquire);
        if !existing.is_null() {
            return existing;
        }

        let len = bucket_len(bucket);
        let mut cells: Vec<AtomicPtr<Slot>> = Vec::with_capacity(len);
        cells.resize_with(len, || AtomicPtr::new(ptr::null_mut()));
        let raw = Box::into_raw(cells.into_boxed_slice()) as *mut AtomicPtr<Slot>;

        match self.buckets[bucket].compare_exchange(
            ptr::null_mut(),
            raw,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => raw,
            Err(winner) => {
                unsafe {
                    drop(Box::from_raw(ptr::slice_from_raw_parts_mut(raw, len)));
                }
                winner
            }
        }
    }

    fn cell(&self, slot_id: SlotId) -> *const AtomicPtr<Slot> {
        let (bucket, offset) = location(slot_id);
        let array = self.buckets[bucket].load(Ordering::Acquire);
        assert!(!array.is_null(), "slot read before allocation");
        unsafe { array.add(offset) }
    }

    fn slot(&self, slot_id: SlotId) -> &Slot {
        let slot = unsafe { &*self.cell(slot_id) }.load(Ordering::Acquire);
        assert!(!slot.is_null(), "slot read before allocation");
        unsafe { &*slot }
    }

    pub fn allocate<T: Send + Sync + 'static>(&self, slot_id: SlotId, initial: T) {
        let (bucket, offset) = location(slot_id);
        let array = self.ensure_bucket(bucket);
        let slot = Box::into_raw(Box::new(Slot::new(Arc::new(initial))));
        let cell = unsafe { &*array.add(offset) };
        let previous = cell.swap(slot, Ordering::AcqRel);
        if !previous.is_null() {
            unsafe {
                drop(Box::from_raw(previous));
            }
        }
    }

    pub fn read_current<T: 'static>(&self, slot_id: SlotId) -> &T {
        let slot = self.slot(slot_id);
        unsafe {
            (*slot.current.get())
                .downcast_ref::<T>()
                .expect("type mismatch on slot read")
        }
    }

    pub fn write_pending<T: Send + Sync + 'static>(&self, slot_id: SlotId, value: T) {
        let slot = self.slot(slot_id);
        unsafe {
            *slot.pending.get() = Arc::new(value);
        }
        slot.dirty.store(true, Ordering::Release);
    }

    pub fn write_pending_raw(&self, slot_id: SlotId, value: Value) {
        let slot = self.slot(slot_id);
        unsafe {
            *slot.pending.get() = value;
        }
        slot.dirty.store(true, Ordering::Release);
    }

    pub fn update_pending<T, F>(&self, slot_id: SlotId, f: F)
    where
        T: Send + Sync + Clone + 'static,
        F: FnOnce(&mut T),
    {
        let slot = self.slot(slot_id);
        let mut value = unsafe {
            let base = if slot.dirty.load(Ordering::Acquire) {
                &*slot.pending.get()
            } else {
                &*slot.current.get()
            };
            base.downcast_ref::<T>()
                .expect("type mismatch on slot update")
                .clone()
        };
        f(&mut value);
        unsafe {
            *slot.pending.get() = Arc::new(value);
        }
        slot.dirty.store(true, Ordering::Release);
    }

    pub fn commit(&self, touched: &[SlotId]) -> Vec<SlotId> {
        let mut committed = Vec::new();
        for &slot_id in touched {
            let slot = self.slot(slot_id);
            if slot.dirty.swap(false, Ordering::AcqRel) {
                unsafe {
                    ptr::swap(slot.current.get(), slot.pending.get());
                }
                committed.push(slot_id);
            }
        }
        committed
    }
}

impl Drop for Slots {
    fn drop(&mut self) {
        for bucket in 0..BUCKETS {
            let array = *self.buckets[bucket].get_mut();
            if array.is_null() {
                continue;
            }
            let len = bucket_len(bucket);
            for offset in 0..len {
                let slot = unsafe { &*array.add(offset) }.load(Ordering::Acquire);
                if !slot.is_null() {
                    unsafe {
                        drop(Box::from_raw(slot));
                    }
                }
            }
            unsafe {
                drop(Box::from_raw(ptr::slice_from_raw_parts_mut(array, len)));
            }
        }
    }
}
