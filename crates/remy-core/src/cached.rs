use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::state::SlotId;
use crate::tracking::{clear_area, is_area_cleared, mark_cleared};
use crate::tracking::{any_slot_dirty, pop_owner, push_owner, OwnerId};
use crate::view::View;

#[derive(Default)]
pub struct ComponentCache {
    pub area: Option<Rect>,
    pub own_slots: HashSet<SlotId>,
    pub child_slots: HashSet<SlotId>,
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
    fn render(&self, buf: &mut Buffer, area: Rect) {
        let rt = crate::runtime::Runtime::get();
        let entry = rt.component_caches.entry(self.owner_id).or_default();

        let own_dirty = any_slot_dirty(&entry.own_slots);
        let child_dirty = any_slot_dirty(&entry.child_slots);
        let was_wiped = is_area_cleared(area);
        let area_unchanged = entry.area == Some(area);

        if !own_dirty && !child_dirty && !was_wiped && area_unchanged {
            return;
        }

        drop(entry);

        if own_dirty || was_wiped {
            clear_area(buf, area);
            mark_cleared(area);
        }

        let prev = crate::tracking::ACTIVE_OWNER.get();
        crate::tracking::ACTIVE_OWNER.set(Some(self.owner_id));
        push_owner();

        self.view.render(buf, area);

        let frame = pop_owner();
        crate::tracking::ACTIVE_OWNER.set(prev);

        let mut entry = rt.component_caches.entry(self.owner_id).or_default();
        entry.area = Some(area);
        entry.own_slots = frame.own.into_iter().collect();
        entry.child_slots = frame.children.into_iter().collect();
    }
}
