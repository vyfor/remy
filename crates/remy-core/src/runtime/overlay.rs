use std::cell::RefCell;
use std::sync::Arc;

use ratatui::layout::Rect;

use crate::cached::{CachedOverlay, OverlayRenderFn};
use crate::tracking::{capture_owner, is_capturing, record_overlay};
use crate::tracking::OwnerId;

pub type OverlayRender = OverlayRenderFn;

pub struct OverlayEntry {
    pub rect: Rect,
    pub render: OverlayRender,
}

thread_local! {
    static OVERLAYS: RefCell<Vec<OverlayEntry>> = const { RefCell::new(Vec::new()) };
}

pub fn push_overlay(rect: Rect, render: OverlayRender) {
    OVERLAYS.with(|o| o.borrow_mut().push(OverlayEntry { rect, render }));
}

pub fn push_overlay_from_cx<F>(owner_id: OwnerId, rect: Rect, render: F)
where
    F: Fn(&mut ratatui::buffer::Buffer, Rect) + Send + Sync + 'static,
{
    let arc: OverlayRender = Arc::new(render);
    push_overlay(rect, arc.clone());

    if is_capturing() && capture_owner() == Some(owner_id) {
        record_overlay(CachedOverlay { rect, render: arc });
    }
}

pub fn drain_overlays() -> Vec<OverlayEntry> {
    OVERLAYS.with(|o| std::mem::take(&mut *o.borrow_mut()))
}

pub fn overlay_rects() -> Vec<Rect> {
    OVERLAYS.with(|o| o.borrow().iter().map(|e| e.rect).collect())
}