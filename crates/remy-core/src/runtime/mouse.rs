use crate::keyboard::Flow;
use crate::mouse::Region;
use crate::tracking::OwnerId;

use super::{FocusId, Runtime, active_capture_id, current_frame_capture_id, focus_id};

pub fn begin_mouse_frame() {
    Runtime::get().mouse.lock().unwrap().begin_frame();
}

pub fn finish_mouse_frame() {
    let rt = Runtime::get();
    let active_cap = active_capture_id();
    let hover_changed = rt.mouse.lock().unwrap().finish_frame(active_cap);
    if hover_changed {
        rt.dirty_notify.notify_one();
    }
}

pub fn add_mouse_region(mut region: Region, owner_id: OwnerId) {
    let cap_id = current_frame_capture_id();
    region.attach_runtime(Some(owner_id), cap_id);
    Runtime::get().mouse.lock().unwrap().register_region(region);
}

pub fn is_region_hovered(id: FocusId) -> bool {
    Runtime::get().mouse.lock().unwrap().is_hovered(id)
}

pub fn dispatch_mouse_event(event: &crossterm::event::MouseEvent) -> Flow {
    let rt = Runtime::get();
    let active_cap = active_capture_id();
    let (result, hover_changed, focus) = rt.mouse.lock().unwrap().dispatch_event(event, active_cap);
    if let Some(focus) = focus {
        focus_id(focus);
    }
    if hover_changed {
        rt.dirty_notify.notify_one();
    }
    result
}
