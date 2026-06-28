use crate::runtime;
use crate::app::App;

use super::State;

pub struct StateInit<T> {
    initial: T,
}

pub fn state<T>(initial: T) -> StateInit<T> {
    StateInit { initial }
}

pub trait Init<Handle> {
    fn install(self, handle: &'static Handle, cx: App);
}

pub fn install<Handle, I>(handle: &'static Handle, init: I, cx: App)
where
    I: Init<Handle>,
{
    init.install(handle, cx);
}

impl<T: Send + Sync + 'static> Init<State<T>> for StateInit<T> {
    fn install(self, handle: &'static State<T>, _cx: App) {
        runtime::allocate_slot(handle.id(), self.initial);
    }
}
