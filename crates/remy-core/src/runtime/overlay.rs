use std::cell::RefCell;

use ratatui::layout::Rect;

pub type OverlayRender = Box<dyn FnOnce(&mut ratatui::Frame, Rect)>;

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

pub fn drain_overlays() -> Vec<OverlayEntry> {
    OVERLAYS.with(|o| std::mem::take(&mut *o.borrow_mut()))
}
