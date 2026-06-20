use crate::focus_builder::{FocusBuilder, FocusGroupBuilder, RenderFocus};
use crate::key::{LiveKeys, StaticKeys};
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

    pub fn keys(&self) -> StaticKeys {
        StaticKeys::new(self.owner_id)
    }

    pub fn configure_keys<F>(&self, f: F)
    where
        F: FnOnce(&mut crate::key::Keys),
    {
        runtime::configure_static_view_keys(self.owner_id, f);
    }

    pub fn focus(self) -> FocusBuilder {
        FocusBuilder::new(self.owner_id)
    }

    pub fn focus_group(self, name: &str) -> FocusGroupBuilder {
        FocusGroupBuilder::new(name, self.owner_id)
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
    pub fn new(owner_id: crate::tracking::OwnerId) -> Self {
        Self { owner_id }
    }

    pub fn keys(&self) -> LiveKeys {
        LiveKeys::new(self.owner_id)
    }

    pub fn focus(self) -> RenderFocus {
        RenderFocus::new(self.owner_id)
    }

    pub fn present(self) {
        runtime::present_focus(self.owner_id)
    }

    pub fn focused(self) -> bool {
        runtime::is_focus_id(FocusId::component(self.owner_id))
    }

    pub fn focus_group<R>(self, name: &str, body: impl FnOnce(&Self) -> R) -> R {
        let group_id = FocusId::new(name);
        runtime::push_group(group_id);
        let result = body(&self);
        runtime::pop_group();
        result
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
