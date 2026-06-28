use crate::runtime;
use crate::scope::Scope;

use super::State;

pub struct StateInit<T> {
    initial: T,
}

pub fn state<T>(initial: T) -> StateInit<T> {
    StateInit { initial }
}

pub trait Init<Handle> {
    fn install(self, handle: &'static Handle, cx: Scope);
}

pub fn install<Handle, I>(handle: &'static Handle, init: I, cx: Scope)
where
    I: Init<Handle>,
{
    init.install(handle, cx);
}

impl<T: Send + Sync + 'static> Init<State<T>> for StateInit<T> {
    fn install(self, handle: &'static State<T>, _cx: Scope) {
        runtime::allocate_slot(handle.id(), self.initial);
    }
}
