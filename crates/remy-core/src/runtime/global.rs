use crate::INTENT_REGISTRY;
use std::sync::OnceLock;

use crate::app::App;

use super::Runtime;

pub fn dispatch_intent(slot: &'static OnceLock<u32>, payload: Box<dyn std::any::Any + Send>) {
    for (id, handler) in INTENT_REGISTRY {
        if std::ptr::eq(*id, slot) {
            id.get_or_init(crate::state::next_slot_id);
            handler(App::new(), payload);
            return;
        }
    }
}

pub fn get_global<T: Send + Sync + Clone + 'static>() -> Option<T> {
    Runtime::get().globals.get::<T>()
}

pub fn get_global_arc<T: Send + Sync + 'static>() -> Option<std::sync::Arc<T>> {
    Runtime::get().globals.get_arc::<T>()
}

pub fn report_error(intent_name: &'static str, error: &dyn std::fmt::Display) {
    eprintln!("[intent {intent_name}] error: {error}");
}
