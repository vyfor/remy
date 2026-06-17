use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::state::SlotId;
use crate::tracking::OwnerId;
use crate::tracking::{any_slot_dirty, pop_owner, push_owner};
use crate::view::View;

#[derive(Default)]
pub struct ComponentCache {
    pub area: Option<Rect>,
    pub buffer: Option<Buffer>,
    pub tracked_slots: HashSet<SlotId>,
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
    fn render(&self, frame: &mut ratatui::Frame, area: Rect) {
        let rt = crate::runtime::Runtime::get();
        let entry = rt.component_caches.entry(self.owner_id).or_default();

        let is_dirty = any_slot_dirty(&entry.tracked_slots);
        let valid = entry.area == Some(area) && !is_dirty;

        if valid {
            if let Some(buffer) = &entry.buffer {
                let dst = frame.buffer_mut();
                for y in 0..area.height {
                    for x in 0..area.width {
                        let src_cell = buffer.cell((x, y)).unwrap();
                        let dst_cell = dst.cell_mut((area.x + x, area.y + y)).unwrap();
                        *dst_cell = src_cell.clone();
                    }
                }
            }
            return;
        }

        drop(entry);

        let prev = crate::tracking::ACTIVE_OWNER.get();
        crate::tracking::ACTIVE_OWNER.set(Some(self.owner_id));
        push_owner();

        self.view.render(frame, area);

        let reads = pop_owner();
        crate::tracking::ACTIVE_OWNER.set(prev);

        let mut captured = Buffer::empty(Rect::new(0, 0, area.width, area.height));
        let src = frame.buffer_mut();
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = src.cell((area.x + x, area.y + y)).unwrap();
                let dst_cell = captured.cell_mut((x, y)).unwrap();
                *dst_cell = cell.clone();
            }
        }

        let mut entry = rt.component_caches.entry(self.owner_id).or_default();
        entry.area = Some(area);
        entry.buffer = Some(captured);
        entry.tracked_slots = reads.iter().copied().collect();
    }
}
