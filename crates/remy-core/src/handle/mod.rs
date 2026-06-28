use std::marker::PhantomData;
use std::ops::Deref;

use crate::runtime;
use crate::state::SlotId;

mod init;

pub use init::{Init, StateInit, install, state};

pub struct State<T: 'static> {
    slot_id: SlotId,
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
    pub const fn new(slot_id: SlotId) -> Self {
        Self {
            slot_id,
            _marker: PhantomData,
        }
    }

    pub const fn id(&self) -> SlotId {
        self.slot_id
    }
}

impl<T: Send + Sync + 'static> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        runtime::track_read(self.slot_id);
        runtime::read_current::<T>(self.slot_id)
    }
}

impl<T: Send + Sync + 'static> State<T> {
    pub fn set(&self, value: T) {
        runtime::write_wake(self.slot_id, value);
    }

    pub fn peek(&self) -> &T {
        runtime::read_current::<T>(self.slot_id)
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(runtime::read_current::<T>(self.slot_id))
    }

    pub fn with_tracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        runtime::track_read(self.slot_id);
        f(runtime::read_current::<T>(self.slot_id))
    }
}

impl<T: Send + Sync + Clone + 'static> State<T> {
    pub fn update(&self, f: impl FnOnce(&mut T) + Send + 'static) {
        runtime::update_wake::<T, _>(self.slot_id, f);
    }
}

impl State<bool> {
    pub fn toggle(&self) {
        let current = *self.peek();
        self.set(!current);
    }
}
