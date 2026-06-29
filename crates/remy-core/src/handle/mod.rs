use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::OnceLock;

use crate::runtime;
use crate::state::SlotId;

mod init;

pub use init::{Init, StateInit, install, state};

pub struct State<T: 'static> {
    slot: &'static OnceLock<SlotId>,
    _marker: PhantomData<T>,
}

impl<T: 'static> Clone for State<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for State<T> {}

unsafe impl<T: 'static> Send for State<T> {}
unsafe impl<T: 'static> Sync for State<T> {}

impl<T: 'static> State<T> {
    pub const fn new(slot: &'static OnceLock<SlotId>) -> Self {
        Self {
            slot,
            _marker: PhantomData,
        }
    }

    pub fn id(&self) -> SlotId {
        *self.slot.get_or_init(runtime::next_slot_id)
    }
}

impl<T: Send + Sync + 'static> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let id = self.id();
        runtime::track_read(id);
        runtime::read_current::<T>(id)
    }
}

impl<T: Send + Sync + 'static> State<T> {
    pub fn set(&self, value: T) {
        runtime::write_wake(self.id(), value);
    }

    pub fn peek(&self) -> &T {
        runtime::read_current::<T>(self.id())
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(runtime::read_current::<T>(self.id()))
    }

    pub fn with_tracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let id = self.id();
        runtime::track_read(id);
        f(runtime::read_current::<T>(id))
    }
}

impl<T: Send + Sync + Clone + 'static> State<T> {
    pub fn update(&self, f: impl FnOnce(&mut T) + Send + 'static) {
        runtime::update_wake::<T, _>(self.id(), f);
    }
}

impl State<bool> {
    pub fn toggle(&self) {
        let current = *self.peek();
        self.set(!current);
    }
}
