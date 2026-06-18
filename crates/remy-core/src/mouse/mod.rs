use std::sync::Arc;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent};
use ratatui::layout::Rect;

use crate::keyboard::{Flow, IntoFlow};
use crate::runtime::{self, FocusId};
use crate::tracking::OwnerId;

mod region;
mod regions;

pub(crate) type MouseAction = Arc<dyn Fn() -> Flow + Send + Sync>;
pub(crate) type ScrollAction = Arc<dyn Fn(Scroll) -> Flow + Send + Sync>;

pub use region::Region;
pub use regions::{DispatchResult, Regions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub column: u16,
    pub row: u16,
    pub modifiers: KeyModifiers,
}

impl Pos {
    pub(crate) fn from_event(event: &MouseEvent) -> Self {
        Self {
            column: event.column,
            row: event.row,
            modifiers: event.modifiers,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scroll {
    pub delta_x: i16,
    pub delta_y: i16,
}

pub struct RegionBuilder {
    region: Option<Region>,
    owner_id: OwnerId,
}

impl RegionBuilder {
    pub(crate) fn for_owner(owner_id: OwnerId, id: impl std::hash::Hash, area: Rect) -> Self {
        Self {
            region: Some(Region::new(FocusId::new(id), area)),
            owner_id,
        }
    }

    pub fn focus_on_click(mut self) -> Self {
        self.region_mut().focus_on_click();
        self
    }

    pub fn on_click<F, R>(mut self, button: MouseButton, handler: F) -> Self
    where
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.region_mut().on_click(button, handler);
        self
    }

    pub fn on_press<F, R>(mut self, button: MouseButton, handler: F) -> Self
    where
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.region_mut().on_press(button, handler);
        self
    }

    pub fn on_release<F, R>(mut self, button: MouseButton, handler: F) -> Self
    where
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.region_mut().on_release(button, handler);
        self
    }

    pub fn on_scroll<F, R>(mut self, handler: F) -> Self
    where
        F: Fn(Scroll) -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.region_mut().on_scroll(handler);
        self
    }

    pub fn hovered(mut self) -> bool {
        let region = self.region.take().expect("mouse region already registered");
        let id = region.id();
        let mut region = region;
        region.wants_hover();
        runtime::add_mouse_region(region, self.owner_id);
        runtime::is_region_hovered(id)
    }

    fn region_mut(&mut self) -> &mut Region {
        self.region
            .as_mut()
            .expect("mouse region already registered")
    }
}

impl Drop for RegionBuilder {
    fn drop(&mut self) {
        if let Some(region) = self.region.take() {
            runtime::add_mouse_region(region, self.owner_id);
        }
    }
}
