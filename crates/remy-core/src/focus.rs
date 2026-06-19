pub use crate::runtime::FocusId;

use crate::keyboard::{Flow, IntoBind, IntoFlow};
use crate::runtime;
use crate::tracking::OwnerId;

pub struct FocusTarget {
    id: FocusId,
    owner_id: OwnerId,
}

pub struct FocusGroup {
    id: FocusId,
    owner_id: OwnerId,
    wrap: bool,
}

#[derive(Clone, Copy)]
pub struct Focus {
    id: FocusId,
    owner_id: OwnerId,
    focused: bool,
}

pub fn current() -> Option<FocusId> {
    runtime::current_focus_id()
}

pub fn active_group() -> Option<FocusId> {
    runtime::active_group()
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

pub fn next_group() -> Flow {
    if runtime::focus_next_group() {
        Flow::Handled
    } else {
        Flow::Ignored
    }
}

pub fn prev_group() -> Flow {
    if runtime::focus_prev_group() {
        Flow::Handled
    } else {
        Flow::Ignored
    }
}

pub fn enter_group() -> Flow {
    if runtime::focus_enter_group() {
        Flow::Handled
    } else {
        Flow::Ignored
    }
}

pub fn leave_group() -> Flow {
    if runtime::focus_leave_group() {
        Flow::Handled
    } else {
        Flow::Ignored
    }
}

pub fn capture<R>(id: &'static str, body: impl FnOnce() -> R) -> R {
    runtime::with_capture(id, body)
}

impl FocusTarget {
    pub(crate) fn new(id: FocusId, owner_id: OwnerId) -> Self {
        Self { id, owner_id }
    }

    pub fn declare(self) -> Focus {
        let focused = runtime::declare_focus(self.id, self.owner_id);
        Focus {
            id: self.id,
            owner_id: self.owner_id,
            focused,
        }
    }

    pub fn group(self) -> FocusGroup {
        runtime::declare_group(self.id, self.owner_id);
        FocusGroup {
            id: self.id,
            owner_id: self.owner_id,
            wrap: true,
        }
    }
}

impl FocusGroup {
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        runtime::set_group_wrap(self.id, wrap);
        self
    }

    pub fn declare(self, child_id: impl std::hash::Hash) -> Focus {
        let child_focus = FocusId::new(child_id);
        let focused = runtime::declare_in_group(self.id, child_focus, self.owner_id);
        Focus {
            id: child_focus,
            owner_id: self.owner_id,
            focused,
        }
    }

    pub fn id(&self) -> FocusId {
        self.id
    }
}

impl Focus {
    pub fn focused(self) -> bool {
        self.focused
    }

    pub fn active(self) -> bool {
        self.focused()
    }

    pub fn on_press<K, F, R>(self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_static_focus_key_press(self.owner_id, self.id, key.into_key_binding(), move || action().into_key_result());
    }

    pub fn on_release<K, F, R>(self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_static_focus_key_release(self.owner_id, self.id, key.into_key_binding(), move || action().into_key_result());
    }

    pub fn on_repeat<K, F, R>(self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_static_focus_key_repeat(self.owner_id, self.id, key.into_key_binding(), move || action().into_key_result());
    }

    pub fn live_on_press<K, F, R>(self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_live_focus_key_press(self.owner_id, self.id, key.into_key_binding(), move || action().into_key_result());
    }

    pub fn live_on_release<K, F, R>(self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_live_focus_key_release(self.owner_id, self.id, key.into_key_binding(), move || action().into_key_result());
    }

    pub fn live_on_repeat<K, F, R>(self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_live_focus_key_repeat(self.owner_id, self.id, key.into_key_binding(), move || action().into_key_result());
    }

    pub fn focus(self) -> Flow {
        if runtime::focus_id(self.id) {
            Flow::Handled
        } else {
            Flow::Ignored
        }
    }

    pub fn id(&self) -> FocusId {
        self.id
    }
}
