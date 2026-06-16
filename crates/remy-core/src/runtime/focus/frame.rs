use crate::runtime::Runtime;

use super::FocusId;

pub fn begin_focus_frame() {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    f.entries.clear();
    f.active_group = None;
    f.groups.clear();
    f.capture_stack.clear();
    f.active_capture = None;
    for cap in f.captures.values_mut() {
        cap.entries.clear();
        cap.current = None;
    }
    drop(f);
    *rt.focused_owner.lock().unwrap() = None;
}

pub fn finish_focus_frame() {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    let mut focused = rt.focused_owner.lock().unwrap();

    if let Some(cap_id) = f.active_capture {
        let Some(cap) = f.captures.get_mut(cap_id) else {
            *focused = None;
            return;
        };

        if let Some(id) = *focused
            && let Some(entry) = cap.entries.iter().find(|entry| entry.owner_id == id)
        {
            cap.desired = Some(entry.id);
            cap.current = Some(entry.id);
            return;
        }

        if cap.entries.is_empty() {
            cap.desired = None;
            cap.current = None;
            *focused = None;
            return;
        }

        let target_id = cap
            .desired
            .and_then(|id| cap.entries.iter().any(|entry| entry.id == id).then_some(id))
            .unwrap_or(cap.entries[0].id);
        let target = cap
            .entries
            .iter()
            .find(|entry| entry.id == target_id)
            .copied()
            .expect("selected capture target disappeared :(");
        cap.desired = Some(target.id);
        cap.current = Some(target.id);
        *focused = Some(target.owner_id);
        return;
    }

    if let Some(id) = *focused {
        let focus_id = f
            .entries
            .iter()
            .find(|entry| entry.owner_id == id)
            .map(|entry| entry.id)
            .unwrap_or_else(|| FocusId::component(id));
        f.desired = Some(focus_id);
        f.current = Some(focus_id);
        return;
    }

    if f.entries.is_empty() {
        f.desired = None;
        f.current = None;
        return;
    }

    let target_id = f
        .desired
        .and_then(|id| f.entries.iter().any(|entry| entry.id == id).then_some(id))
        .unwrap_or(f.entries[0].id);
    let target = f
        .entries
        .iter()
        .find(|entry| entry.id == target_id)
        .copied()
        .expect("targetn't");
    f.desired = Some(target.id);
    f.current = Some(target.id);
    *focused = Some(target.owner_id);
}
