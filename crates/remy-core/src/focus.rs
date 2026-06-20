pub use crate::focus_builder::{FocusBuilder, FocusGroupBuilder, RenderFocus};
pub use crate::runtime::FocusId;

use crate::keyboard::Flow;
use crate::runtime;

pub fn current() -> Option<FocusId> {
    runtime::current_focus_id()
}

pub fn is(id: impl std::hash::Hash) -> bool {
    runtime::is_focus_id(FocusId::new(id))
}

pub fn set(id: impl std::hash::Hash) -> Flow {
    if runtime::focus_id(FocusId::new(id)) {
        Flow::Handled
    } else {
        Flow::Ignored
    }
}

pub fn clear() -> Flow {
    runtime::clear_focus();
    Flow::Handled
}

pub fn next() -> Flow {
    if runtime::focus_next() {
        Flow::Handled
    } else {
        Flow::Ignored
    }
}

pub fn prev() -> Flow {
    if runtime::focus_prev() {
        Flow::Handled
    } else {
        Flow::Ignored
    }
}

pub fn trap<R>(id: &'static str, body: impl FnOnce() -> R) -> R {
    runtime::push_trap(id);
    let result = body();
    runtime::pop_trap();
    result
}
