use std::cell::{Cell, RefCell};
use std::collections::HashSet;

use ratatui::layout::{Position, Rect};

use crate::cached::{CachedMouseRegion, CachedOverlay};
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

    pub(crate) static CAPTURING: Cell<bool> = const { Cell::new(false) };
    pub(crate) static CAPTURE_OWNER: Cell<Option<OwnerId>> = const { Cell::new(None) };
    pub(crate) static DECLARED_REGIONS: RefCell<Vec<CachedMouseRegion>> =
        const { RefCell::new(Vec::new()) };
    pub(crate) static DECLARED_OVERLAYS: RefCell<Vec<CachedOverlay>> =
        const { RefCell::new(Vec::new()) };
}

#[derive(Default)]
pub struct OwnerFrame {
    pub own: Vec<SlotId>,
    pub children: Vec<SlotId>,
    pub rendered_children: Vec<OwnerId>,
}

pub struct DeclarationCaptures {
    pub regions: Vec<CachedMouseRegion>,
    pub overlays: Vec<CachedOverlay>,
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
            parent
                .rendered_children
                .extend(frame.rendered_children.iter().copied());
        }
    });
    frame
}

pub fn record_child(child_id: OwnerId) {
    OWNER_STACK.with(|stack| {
        if let Some(top) = stack.borrow_mut().last_mut()
            && !top.rendered_children.contains(&child_id)
        {
            top.rendered_children.push(child_id);
        }
    });
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

pub fn clear_declarations() {
    DECLARED_REGIONS.with(|r| r.borrow_mut().clear());
    DECLARED_OVERLAYS.with(|o| o.borrow_mut().clear());
}

pub fn begin_declaration_capture(owner: OwnerId) {
    CAPTURING.set(true);
    CAPTURE_OWNER.set(Some(owner));
}

pub fn end_declaration_capture() {
    CAPTURING.set(false);
    CAPTURE_OWNER.set(None);
}

pub fn is_capturing() -> bool {
    CAPTURING.get()
}

pub fn capture_owner() -> Option<OwnerId> {
    CAPTURE_OWNER.get()
}

pub fn record_mouse_region(region: CachedMouseRegion) {
    DECLARED_REGIONS.with(|r| r.borrow_mut().push(region));
}

pub fn record_overlay(overlay: CachedOverlay) {
    DECLARED_OVERLAYS.with(|o| o.borrow_mut().push(overlay));
}

pub fn drain_declarations() -> DeclarationCaptures {
    let regions = DECLARED_REGIONS.with(|r| std::mem::take(&mut *r.borrow_mut()));
    let overlays = DECLARED_OVERLAYS.with(|o| std::mem::take(&mut *o.borrow_mut()));
    DeclarationCaptures { regions, overlays }
}
