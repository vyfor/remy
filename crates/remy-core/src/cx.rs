use crate::focus::FocusTarget;
use crate::key::Keys;
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

    pub fn keys(self, configure: impl FnOnce(&mut Keys)) {
        let mut keys = Keys::new();
        configure(&mut keys);
        runtime::add_view_keys(self.owner_id, keys);
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
        render: impl Fn(Cx, &mut ratatui::buffer::Buffer, ratatui::layout::Rect) + Send + Sync + 'static,
    ) {
        let owner_id = self.owner_id;
        runtime::push_overlay_from_cx(self.owner_id, rect, move |buf, area| {
            let cx = Cx::new(owner_id);
            render(cx, buf, area);
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
