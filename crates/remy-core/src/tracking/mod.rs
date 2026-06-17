use std::cell::{Cell, RefCell};
use std::collections::HashSet;

use ratatui::layout::{Position, Rect};

use crate::effect::EffectId;
use crate::state::SlotId;

mod effect;

pub use effect::{EffectGuard, current_effect, effect_context};

pub type OwnerId = u32;

thread_local! {
    pub(crate) static ACTIVE_EFFECT: Cell<Option<EffectId>> = const { Cell::new(None) };
    pub(crate) static ACTIVE_OWNER: Cell<Option<OwnerId>> = const { Cell::new(None) };
    pub(crate) static RENDER_TRACKING: Cell<bool> = const { Cell::new(false) };
    pub(crate) static RENDER_READS: RefCell<Vec<SlotId>> = const { RefCell::new(Vec::new()) };
    pub(crate) static OWNER_STACK: RefCell<Vec<OwnerFrame>> = const { RefCell::new(Vec::new()) };
    pub(crate) static DIRTY_SLOTS: RefCell<Vec<SlotId>> = const { RefCell::new(Vec::new()) };
    pub(crate) static CLEARED_AREAS: RefCell<Vec<Rect>> = const { RefCell::new(Vec::new()) };
    pub(crate) static CURSOR_POSITION: Cell<Option<Position>> = const { Cell::new(None) };
}

#[derive(Default)]
pub struct OwnerFrame {
    pub own: Vec<SlotId>,
    pub children: Vec<SlotId>,
}

pub fn begin_render_tracking() {
    RENDER_TRACKING.set(true);
}

pub fn end_render_tracking() {
    RENDER_TRACKING.set(false);
}

pub fn is_render_tracking() -> bool {
    RENDER_TRACKING.get()
}

pub fn record_render_read(slot_id: SlotId) {
    RENDER_READS.with(|reads| reads.borrow_mut().push(slot_id));
    OWNER_STACK.with(|stack| {
        if let Some(top) = stack.borrow_mut().last_mut() {
            top.own.push(slot_id);
        }
    });
}

pub fn drain_render_reads() -> Vec<SlotId> {
    RENDER_READS.with(|reads| std::mem::take(&mut *reads.borrow_mut()))
}

pub fn push_owner() {
    OWNER_STACK.with(|stack| stack.borrow_mut().push(OwnerFrame::default()));
}

pub fn pop_owner() -> OwnerFrame {
    let frame = OWNER_STACK
        .with(|stack| stack.borrow_mut().pop())
        .unwrap_or_default();
    OWNER_STACK.with(|stack| {
        if let Some(parent) = stack.borrow_mut().last_mut() {
            parent.children.extend(frame.own.iter().copied());
            parent.children.extend(frame.children.iter().copied());
        }
    });
    frame
}

pub fn set_dirty_slots(slots: Vec<SlotId>) {
    DIRTY_SLOTS.with(|d| *d.borrow_mut() = slots);
}

pub fn any_slot_dirty(slots: &HashSet<SlotId>) -> bool {
    DIRTY_SLOTS.with(|d| d.borrow().iter().any(|s| slots.contains(s)))
}

pub fn mark_cleared(area: Rect) {
    CLEARED_AREAS.with(|areas| areas.borrow_mut().push(area));
}

pub fn take_cleared_areas() -> Vec<Rect> {
    CLEARED_AREAS.with(|areas| std::mem::take(&mut *areas.borrow_mut()))
}

pub fn is_area_cleared(area: Rect) -> bool {
    CLEARED_AREAS.with(|areas| {
        areas
            .borrow()
            .iter()
            .any(|cleared| intersects(*cleared, area))
    })
}

fn intersects(a: Rect, b: Rect) -> bool {
    let x_overlap = a.x < b.x.saturating_add(b.width) && a.x.saturating_add(a.width) > b.x;
    let y_overlap = a.y < b.y.saturating_add(b.height) && a.y.saturating_add(a.height) > b.y;
    x_overlap && y_overlap
}

pub fn set_cursor_position(position: Option<Position>) {
    CURSOR_POSITION.set(position);
}

pub fn take_cursor_position() -> Option<Position> {
    CURSOR_POSITION.replace(None)
}

pub fn clear_area(buf: &mut ratatui::buffer::Buffer, area: Rect) {
    if area.is_empty() {
        return;
    }
    let buf_area = buf.area;
    let clip = area.intersection(buf_area);
    if clip.is_empty() {
        return;
    }
    let width = buf_area.width as usize;
    let x = clip.x as usize - buf_area.x as usize;
    let y = clip.y as usize - buf_area.y as usize;
    let w = clip.width as usize;
    let h = clip.height as usize;
    for row in 0..h {
        let start = (y + row) * width + x;
        buf.content[start..start + w].fill(ratatui::buffer::Cell::EMPTY);
    }
}
