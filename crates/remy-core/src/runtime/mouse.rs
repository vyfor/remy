use crate::cached::CachedMouseRegion;
use crate::keyboard::Flow;
use crate::mouse::Region;
use crate::tracking::OwnerId;

use super::{FocusId, Runtime, active_trap_id, current_frame_trap_id, focus_owner};

pub fn begin_mouse_frame() {
    Runtime::get().mouse.lock().unwrap().begin_frame();
}

pub fn finish_mouse_frame() {
    let rt = Runtime::get();
    let active = active_trap_id();
    let (hover_changed, hovered_owners) = rt.mouse.lock().unwrap().finish_frame(active);
    for owner in &hovered_owners {
        mark_mouse_dirty(*owner);
    }
    if hover_changed {
        rt.mouse_changed
            .store(true, std::sync::atomic::Ordering::Relaxed);
        rt.dirty_notify.notify_one();
    }
}

pub fn add_mouse_region(mut region: Region, owner_id: OwnerId) {
    let trap_id = current_frame_trap_id();
    region.attach_runtime(Some(owner_id), trap_id);

    if crate::tracking::is_capturing() && crate::tracking::capture_owner() == Some(owner_id) {
        let cached = crate::cached::CachedMouseRegion {
            id: region.id,
            area: region.area,
            wants_hover: region.wants_hover,
            focus_on_click: region.focus_on_click,
            on_click: region.clicks.clone(),
            on_press: region.presses.clone(),
            on_release: region.releases.clone(),
            on_scroll: region.scroll.clone(),
        };
        crate::tracking::record_mouse_region(cached);
    }

    Runtime::get().mouse.lock().unwrap().register_region(region);
}

pub fn replay_mouse_region(owner_id: OwnerId, region: &CachedMouseRegion) {
    let mut r = Region::new(region.id, region.area);
    r.owner_id = Some(owner_id);
    r.focus_on_click = region.focus_on_click;
    r.wants_hover = region.wants_hover;
    r.clicks = region.on_click.clone();
    r.presses = region.on_press.clone();
    r.releases = region.on_release.clone();
    r.scroll = region.on_scroll.clone();
    add_mouse_region(r, owner_id);
}

pub fn is_region_hovered(id: FocusId) -> bool {
    Runtime::get().mouse.lock().unwrap().is_hovered(id)
}

pub fn dispatch_mouse_event(event: &crossterm::event::MouseEvent) -> Flow {
    let rt = Runtime::get();
    let active = active_trap_id();
    let (dispatch_result, hover_changed, focus_owner_id) = {
        let mut mouse = rt.mouse.lock().unwrap();
        mouse.dispatch_event(event, active)
    };

    if let Some(owner_id) = focus_owner_id {
        focus_owner(owner_id);
    }

    let result = match dispatch_result {
        crate::mouse::DispatchResult::None => Flow::Ignored,
        crate::mouse::DispatchResult::Single(action) => action(),
        crate::mouse::DispatchResult::Multiple(actions) => {
            let mut result = Flow::Ignored;
            for action in actions {
                let r = action();
                result = match (result, r) {
                    (Flow::Quit, _) | (_, Flow::Quit) => Flow::Quit,
                    (Flow::Handled, _) | (_, Flow::Handled) => Flow::Handled,
                    (Flow::Ignored, Flow::Ignored) => Flow::Ignored,
                };
            }
            result
        }
    };

    if hover_changed {
        rt.dirty_notify.notify_one();
    }
    result
}

pub fn mark_mouse_dirty(owner: OwnerId) {
    let rt = Runtime::get();
    let mut current = Some(owner);
    while let Some(id) = current {
        if let Some(mut entry) = rt.component_caches.get_mut(&id) {
            entry.mouse_dirty = true;
            current = entry.parent;
        } else {
            break;
        }
    }
}

pub fn is_mouse_dirty(owner: OwnerId) -> bool {
    Runtime::get()
        .component_caches
        .get(&owner)
        .map(|e| e.mouse_dirty)
        .unwrap_or(false)
}

pub fn clear_mouse_dirty(owner: OwnerId) {
    if let Some(mut entry) = Runtime::get().component_caches.get_mut(&owner) {
        entry.mouse_dirty = false;
    }
}
