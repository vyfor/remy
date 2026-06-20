use std::collections::HashSet;
use std::sync::Arc;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::cx::Rcx;
use crate::mouse::{MouseAction, ScrollAction};
use crate::runtime::FocusId;
use crate::state::SlotId;
use crate::tracking::{OwnerId, any_slot_dirty, pop_owner, push_owner};
use crate::tracking::{clear_area, is_area_cleared, mark_cleared};
use crate::view::View;

#[derive(Clone)]
pub struct CachedMouseRegion {
    pub id: FocusId,
    pub area: Rect,
    pub wants_hover: bool,
    pub focus_on_click: bool,
    pub on_click: Vec<(crossterm::event::MouseButton, MouseAction)>,
    pub on_press: Vec<(crossterm::event::MouseButton, MouseAction)>,
    pub on_release: Vec<(crossterm::event::MouseButton, MouseAction)>,
    pub on_scroll: Option<ScrollAction>,
}

pub type OverlayRenderFn = Arc<dyn Fn(&mut Buffer, Rect) + Send + Sync>;

#[derive(Clone)]
pub struct CachedOverlay {
    pub rect: Rect,
    pub render: OverlayRenderFn,
}

pub struct ComponentCache {
    pub area: Option<Rect>,
    pub own_slots: HashSet<SlotId>,
    pub child_slots: HashSet<SlotId>,
    pub mouse_regions: Arc<[CachedMouseRegion]>,
    pub overlays: Arc<[CachedOverlay]>,
    pub mouse_dirty: bool,
    pub parent: Option<OwnerId>,
    pub children: Arc<[OwnerId]>,
}

impl Default for ComponentCache {
    fn default() -> Self {
        Self {
            area: None,
            own_slots: HashSet::new(),
            child_slots: HashSet::new(),
            mouse_regions: Arc::from([]),
            overlays: Arc::from([]),
            mouse_dirty: false,
            parent: None,
            children: Arc::from([]),
        }
    }
}

pub struct CachedView<V> {
    owner_id: OwnerId,
    view: V,
}

impl<V: View> CachedView<V> {
    pub fn new(owner_id: OwnerId, view: V) -> Self {
        Self { owner_id, view }
    }
}

impl<V: View> View for CachedView<V> {
    fn render(&self, _rcx: Rcx, buf: &mut Buffer, area: Rect) {
        let rt = crate::runtime::Runtime::get();

        rt.static_seen.lock().unwrap().insert(self.owner_id);

        let mut entry = rt.component_caches.entry(self.owner_id).or_default();

        let own_dirty = any_slot_dirty(&entry.own_slots);
        let child_dirty = any_slot_dirty(&entry.child_slots);
        let mouse_dirty = entry.mouse_dirty;
        let was_wiped = is_area_cleared(area);
        let area_unchanged = entry.area == Some(area);

        if !own_dirty && !child_dirty && !mouse_dirty && !was_wiped && area_unchanged {
            if entry.mouse_regions.is_empty()
                && entry.overlays.is_empty()
                && entry.children.is_empty()
            {
                drop(entry);
                crate::runtime::present_focus(self.owner_id);
                return;
            }
            entry.mouse_dirty = false;
            let regions = Arc::clone(&entry.mouse_regions);
            let overlays = Arc::clone(&entry.overlays);
            let children = Arc::clone(&entry.children);
            drop(entry);

            crate::runtime::present_focus(self.owner_id);
            for region in regions.iter() {
                crate::runtime::replay_mouse_region(self.owner_id, region);
            }
            for overlay in overlays.iter() {
                crate::runtime::push_overlay(overlay.rect, Arc::clone(&overlay.render));
            }
            for child_id in children.iter() {
                replay_subtree(*child_id);
            }
            return;
        }

        drop(entry);

        if own_dirty || was_wiped {
            clear_area(buf, area);
            mark_cleared(area);
        }

        crate::tracking::clear_declarations();
        crate::tracking::begin_declaration_capture(self.owner_id);

        let parent_id = crate::tracking::ACTIVE_OWNER.get();
        crate::tracking::record_child(self.owner_id);

        let prev = crate::tracking::ACTIVE_OWNER.get();
        crate::tracking::ACTIVE_OWNER.set(Some(self.owner_id));
        push_owner();

        self.view.render(Rcx::new(self.owner_id), buf, area);

        let frame = pop_owner();
        crate::tracking::ACTIVE_OWNER.set(prev);

        crate::tracking::end_declaration_capture();

        let captures = crate::tracking::drain_declarations();
        let mut entry = rt.component_caches.entry(self.owner_id).or_default();
        entry.area = Some(area);
        entry.own_slots = frame.own.into_iter().collect();
        entry.child_slots = frame.children.into_iter().collect();
        entry.mouse_regions = Arc::from(captures.regions);
        entry.overlays = Arc::from(captures.overlays);
        entry.mouse_dirty = false;
        entry.parent = parent_id;
        entry.children = Arc::from(frame.rendered_children);
    }
}

fn replay_subtree(owner_id: OwnerId) {
    let (regions, overlays, children) = {
        let rt = crate::runtime::Runtime::get();
        let entry = match rt.component_caches.get(&owner_id) {
            Some(e) => e,
            None => return,
        };
        (
            Arc::clone(&entry.mouse_regions),
            Arc::clone(&entry.overlays),
            Arc::clone(&entry.children),
        )
    };

    for region in regions.iter() {
        crate::runtime::replay_mouse_region(owner_id, region);
    }
    for overlay in overlays.iter() {
        crate::runtime::push_overlay(overlay.rect, Arc::clone(&overlay.render));
    }
    for child_id in children.iter() {
        replay_subtree(*child_id);
    }
}
