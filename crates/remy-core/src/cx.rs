use crate::focus::FocusTarget;
use crate::keyboard::{Flow, IntoBind, IntoFlow};
use crate::mouse::RegionBuilder;
use crate::runtime::{self, FocusId};

#[derive(Clone, Copy)]
pub struct Cx {
    owner_id: crate::tracking::OwnerId,
}

impl Cx {
    pub const fn new(owner_id: crate::tracking::OwnerId) -> Self {
        Self { owner_id }
    }

    pub fn on_press<K, F, R>(&self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_static_view_key_press(self.owner_id, key.into_key_binding(), move || {
            action().into_key_result()
        });
    }

    pub fn on_release<K, F, R>(&self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_static_view_key_release(self.owner_id, key.into_key_binding(), move || {
            action().into_key_result()
        });
    }

    pub fn on_repeat<K, F, R>(&self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_static_view_key_repeat(self.owner_id, key.into_key_binding(), move || {
            action().into_key_result()
        });
    }

    pub fn on_press_any<I, K, F, R>(&self, keys: I, action: F)
    where
        I: IntoIterator<Item = K>,
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        use std::sync::Arc;
        let action: Arc<dyn Fn() -> Flow + Send + Sync> =
            Arc::new(move || action().into_key_result());
        for key in keys {
            let action = Arc::clone(&action);
            runtime::add_static_view_key_press_arc(self.owner_id, key.into_key_binding(), action);
        }
    }

    pub fn focus(self, id: impl std::hash::Hash) -> FocusTarget {
        FocusTarget::new(FocusId::new(id), self.owner_id)
    }

    pub fn mouse_region(
        self,
        id: impl std::hash::Hash,
        area: ratatui::layout::Rect,
    ) -> RegionBuilder {
        RegionBuilder::for_owner(self.owner_id, id, area)
    }

    pub fn overlay(
        self,
        rect: ratatui::layout::Rect,
        render: impl Fn(Rcx, &mut ratatui::buffer::Buffer, ratatui::layout::Rect)
            + Send
            + Sync
            + 'static,
    ) {
        let owner_id = self.owner_id;
        runtime::push_overlay_from_cx(self.owner_id, rect, move |buf, area| {
            let rcx = Rcx::new(owner_id);
            render(rcx, buf, area);
        });
    }

    pub fn global<T: Send + Sync + Clone + 'static>(self) -> T {
        runtime::get_global::<T>()
            .unwrap_or_else(|| panic!("global of type {} not provided", std::any::type_name::<T>()))
    }

    pub fn try_global<T: Send + Sync + Clone + 'static>(self) -> Option<T> {
        runtime::get_global::<T>()
    }

    pub fn global_arc<T: Send + Sync + 'static>(self) -> std::sync::Arc<T> {
        runtime::get_global_arc::<T>()
            .unwrap_or_else(|| panic!("global of type {} not provided", std::any::type_name::<T>()))
    }

    pub fn try_global_arc<T: Send + Sync + 'static>(self) -> Option<std::sync::Arc<T>> {
        runtime::get_global_arc::<T>()
    }
}

#[derive(Clone, Copy)]
pub struct Rcx {
    owner_id: crate::tracking::OwnerId,
}

impl Rcx {
    pub(crate) fn new(owner_id: crate::tracking::OwnerId) -> Self {
        Self { owner_id }
    }

    pub fn on_press<K, F, R>(&self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_live_view_key_press(self.owner_id, key.into_key_binding(), move || {
            action().into_key_result()
        });
    }

    pub fn on_release<K, F, R>(&self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_live_view_key_release(self.owner_id, key.into_key_binding(), move || {
            action().into_key_result()
        });
    }

    pub fn on_repeat<K, F, R>(&self, key: K, action: F)
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        runtime::add_live_view_key_repeat(self.owner_id, key.into_key_binding(), move || {
            action().into_key_result()
        });
    }

    pub fn on_press_any<I, K, F, R>(&self, keys: I, action: F)
    where
        I: IntoIterator<Item = K>,
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        use std::sync::Arc;
        let action: Arc<dyn Fn() -> Flow + Send + Sync> =
            Arc::new(move || action().into_key_result());
        for key in keys {
            let action = Arc::clone(&action);
            runtime::add_live_view_key_press_arc(self.owner_id, key.into_key_binding(), action);
        }
    }

    pub fn focus(self, id: impl std::hash::Hash) -> FocusTarget {
        FocusTarget::new(FocusId::new(id), self.owner_id)
    }

    pub fn mouse_region(
        self,
        id: impl std::hash::Hash,
        area: ratatui::layout::Rect,
    ) -> RegionBuilder {
        RegionBuilder::for_owner(self.owner_id, id, area)
    }

    pub fn overlay(
        self,
        rect: ratatui::layout::Rect,
        render: impl Fn(Rcx, &mut ratatui::buffer::Buffer, ratatui::layout::Rect)
            + Send
            + Sync
            + 'static,
    ) {
        let owner_id = self.owner_id;
        runtime::push_overlay_from_cx(self.owner_id, rect, move |buf, area| {
            let rcx = Rcx::new(owner_id);
            render(rcx, buf, area);
        });
    }

    pub fn global<T: Send + Sync + Clone + 'static>(self) -> T {
        runtime::get_global::<T>()
            .unwrap_or_else(|| panic!("global of type {} not provided", std::any::type_name::<T>()))
    }

    pub fn try_global<T: Send + Sync + Clone + 'static>(self) -> Option<T> {
        runtime::get_global::<T>()
    }

    pub fn global_arc<T: Send + Sync + 'static>(self) -> std::sync::Arc<T> {
        runtime::get_global_arc::<T>()
            .unwrap_or_else(|| panic!("global of type {} not provided", std::any::type_name::<T>()))
    }

    pub fn try_global_arc<T: Send + Sync + 'static>(self) -> Option<std::sync::Arc<T>> {
        runtime::get_global_arc::<T>()
    }
}
