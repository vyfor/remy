use std::marker::PhantomData;
use std::ops::Deref;

use crate::runtime;
use crate::state::SlotId;

mod init;
mod state;

pub use init::{Init, StateInit, install, state};
pub use state::State;

pub struct Proxy<T: 'static> {
    slot_id: SlotId,
    _marker: PhantomData<T>,
}

impl<T: 'static> Clone for Proxy<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Proxy<T> {}

unsafe impl<T: 'static> Send for Proxy<T> {}
unsafe impl<T: 'static> Sync for Proxy<T> {}

impl<T: 'static> Proxy<T> {
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

impl<T: Send + Sync + 'static> Deref for Proxy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let id = self.slot_id;
        runtime::track_read(id);
        runtime::read_current::<T>(id)
    }
}

impl<T: Send + Sync + 'static> Proxy<T> {
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

impl<T: Send + Sync + Clone + 'static> Proxy<T> {
    pub fn update(&self, f: impl FnOnce(&mut T) + Send + 'static) {
        runtime::update_wake::<T, _>(self.slot_id, f);
    }
}

impl Proxy<bool> {
    pub fn toggle(&self) {
        let current = *self.peek();
        self.set(!current);
    }
}
