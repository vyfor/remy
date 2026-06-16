use std::ops::Deref;

use super::Proxy;
use crate::state::SlotId;

pub struct State<T: 'static> {
    proxy: Proxy<T>,
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
            proxy: Proxy::new(slot_id),
        }
    }

    pub const fn id(&self) -> SlotId {
        self.proxy.id()
    }

    pub const fn proxy(&self) -> Proxy<T> {
        self.proxy
    }
}

impl<T: 'static> From<State<T>> for Proxy<T> {
    fn from(state: State<T>) -> Self {
        state.proxy
    }
}

impl<T: Send + Sync + 'static> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.proxy.deref()
    }
}

impl<T: Send + Sync + 'static> State<T> {
    pub fn set(&self, value: T) {
        self.proxy.set(value);
    }

    #[track_caller]
    pub fn peek(&self) -> &T {
        self.proxy.peek()
    }

    #[track_caller]
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.proxy.with(f)
    }

    pub fn with_tracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.proxy.with_tracked(f)
    }
}

impl<T: Send + Sync + Clone + 'static> State<T> {
    pub fn update(&self, f: impl FnOnce(&mut T) + Send + 'static) {
        self.proxy.update(f);
    }
}

impl State<bool> {
    pub fn toggle(&self) {
        self.proxy.toggle();
    }
}
