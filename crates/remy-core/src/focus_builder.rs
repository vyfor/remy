use std::sync::Arc;

use crate::keyboard::{IntoBind, IntoFlow};
use crate::runtime::{self, FocusId};
use crate::tracking::OwnerId;
use crate::id::Id;

pub struct FocusBuilder {
    id: FocusId,
    owner_id: OwnerId,
}

impl FocusBuilder {
    pub(crate) fn new(owner_id: OwnerId) -> Self {
        let id = FocusId::component(owner_id);
        Self { id, owner_id }
    }

    pub fn on_press<K, F, R>(self, key: K, action: F) -> Self
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_static_focus_key_press(
            self.owner_id,
            self.id,
            key.into_key_binding(),
            move || action().into_key_result(),
        );
        self
    }

    pub fn on_release<K, F, R>(self, key: K, action: F) -> Self
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_static_focus_key_release(
            self.owner_id,
            self.id,
            key.into_key_binding(),
            move || action().into_key_result(),
        );
        self
    }

    pub fn on_repeat<K, F, R>(self, key: K, action: F) -> Self
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_static_focus_key_repeat(
            self.owner_id,
            self.id,
            key.into_key_binding(),
            move || action().into_key_result(),
        );
        self
    }

    pub fn on_focus<F>(self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        runtime::add_focus_event(self.id, self.owner_id, FocusEventKind::Focus, callback);
        self
    }

    pub fn on_blur<F>(self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        runtime::add_focus_event(self.id, self.owner_id, FocusEventKind::Blur, callback);
        self
    }
}

#[derive(Clone, Copy)]
pub struct RenderFocus {
    id: FocusId,
    owner_id: OwnerId,
}

impl RenderFocus {
    pub(crate) fn new(owner_id: OwnerId) -> Self {
        let id = FocusId::component(owner_id);
        Self { id, owner_id }
    }

    pub fn focused(self) -> bool {
        runtime::is_focus_id(self.id)
    }

    pub fn on_press<K, F, R>(self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_live_focus_key_press(
            self.owner_id,
            self.id,
            key.into_key_binding(),
            move || action().into_key_result(),
        );
    }

    pub fn on_release<K, F, R>(self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_live_focus_key_release(
            self.owner_id,
            self.id,
            key.into_key_binding(),
            move || action().into_key_result(),
        );
    }

    pub fn on_repeat<K, F, R>(self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_live_focus_key_repeat(
            self.owner_id,
            self.id,
            key.into_key_binding(),
            move || action().into_key_result(),
        );
    }
}

pub struct FocusGroupBuilder {
    group_id: FocusId,
    owner_id: OwnerId,
    wrap: bool,
}

impl FocusGroupBuilder {
    pub(crate) fn new(name: &str, owner_id: OwnerId) -> Self {
        Self {
            group_id: FocusId::new(name),
            owner_id,
            wrap: true,
        }
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn member(self, id: impl Into<Id>) -> Self {
        let member_id = FocusId::new(id.into());
        runtime::add_static_group_member(self.group_id, self.owner_id, member_id, self.wrap);
        self
    }
}

pub enum FocusEventKind {
    Focus,
    Blur,
}

pub(crate) type EventCallback = Arc<dyn Fn() + Send + Sync>;
