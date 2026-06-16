use crate::runtime::Runtime;
use crate::tracking::OwnerId;

use super::FocusId;
use super::state::FocusEntry;

pub fn focus_owner(owner_id: OwnerId) {
    let rt = Runtime::get();
    let focus_id = FocusId::component(owner_id);
    let mut f = rt.focus.lock().unwrap();
    if let Some(cap_id) = f.capture_stack.last().copied() {
        let cap = f.captures.entry(cap_id).or_default();
        cap.desired = Some(focus_id);
        cap.current = Some(focus_id);
        f.active_capture = Some(cap_id);
    } else {
        f.desired = Some(focus_id);
        f.current = Some(focus_id);
    }
    drop(f);
    *rt.focused_owner.lock().unwrap() = Some(owner_id);
}

pub fn declare_focus(focus_id: FocusId, owner_id: OwnerId) -> bool {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();

    if let Some(cap_id) = f.capture_stack.last().copied() {
        f.active_capture = Some(cap_id);
        let cap = f.captures.entry(cap_id).or_default();
        if !cap.entries.iter().any(|entry| entry.id == focus_id) {
            cap.entries.push(FocusEntry {
                id: focus_id,
                owner_id,
            });
        }

        let should_focus = cap.desired == Some(focus_id)
            || (cap.desired.is_none()
                && cap.entries.first().map(|entry| entry.id) == Some(focus_id));
        if should_focus {
            cap.desired = Some(focus_id);
            cap.current = Some(focus_id);
            *rt.focused_owner.lock().unwrap() = Some(owner_id);
        }
        return should_focus;
    }

    if !f.entries.iter().any(|entry| entry.id == focus_id) {
        f.entries.push(FocusEntry {
            id: focus_id,
            owner_id,
        });
    }

    let should_focus = f.desired == Some(focus_id)
        || (f.desired.is_none() && f.entries.first().map(|entry| entry.id) == Some(focus_id));
    if should_focus {
        f.desired = Some(focus_id);
        f.current = Some(focus_id);
        *rt.focused_owner.lock().unwrap() = Some(owner_id);
    }
    should_focus
}

pub fn current_focus_id() -> Option<FocusId> {
    let rt = Runtime::get();
    let f = rt.focus.lock().unwrap();
    if let Some(cap_id) = f.active_capture
        && let Some(cap) = f.captures.get(cap_id)
        && cap.current.is_some()
    {
        return cap.current;
    }
    f.current
}

pub fn is_focus_id(focus_id: FocusId) -> bool {
    current_focus_id() == Some(focus_id)
}

pub fn focus_id(focus_id: FocusId) -> bool {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();

    if let Some(cap_id) = f.active_capture
        && let Some(cap) = f.captures.get_mut(cap_id)
        && let Some(entry) = cap
            .entries
            .iter()
            .find(|entry| entry.id == focus_id)
            .copied()
    {
        cap.desired = Some(focus_id);
        cap.current = Some(focus_id);
        *rt.focused_owner.lock().unwrap() = Some(entry.owner_id);
        rt.dirty_notify.notify_one();
        return true;
    }

    if let Some(entry) = f.entries.iter().find(|entry| entry.id == focus_id).copied() {
        f.desired = Some(focus_id);
        f.current = Some(focus_id);
        *rt.focused_owner.lock().unwrap() = Some(entry.owner_id);
        rt.dirty_notify.notify_one();
        return true;
    }

    f.desired = Some(focus_id);
    false
}

pub fn get_focused_owner() -> Option<OwnerId> {
    let rt = Runtime::get();
    *rt.focused_owner.lock().unwrap()
}

pub fn declare_group(group_id: FocusId, owner_id: OwnerId) {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();

    if !f.entries.iter().any(|entry| entry.id == group_id) {
        f.entries.push(FocusEntry {
            id: group_id,
            owner_id,
        });
    }

    let group = f.groups.entry(group_id).or_default();
    group.owner_id = owner_id;
    group.wrap = true;
}

pub fn declare_in_group(group_id: FocusId, child_id: FocusId, owner_id: OwnerId) -> bool {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();

    if !f.entries.iter().any(|entry| entry.id == child_id) {
        f.entries.push(FocusEntry {
            id: child_id,
            owner_id,
        });
    }

    let group = f.groups.entry(group_id).or_default();
    if !group.entries.iter().any(|entry| entry.id == child_id) {
        group.entries.push(FocusEntry {
            id: child_id,
            owner_id,
        });
    }

    let should_focus = f.desired == Some(child_id)
        || (f.desired.is_none() && f.entries.first().map(|entry| entry.id) == Some(child_id));
    if should_focus {
        f.desired = Some(child_id);
        f.current = Some(child_id);
        *rt.focused_owner.lock().unwrap() = Some(owner_id);
    }
    should_focus
}

pub fn set_group_wrap(group_id: FocusId, wrap: bool) {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    if let Some(group) = f.groups.get_mut(&group_id) {
        group.wrap = wrap;
    }
}

pub fn active_group() -> Option<FocusId> {
    let rt = Runtime::get();
    let f = rt.focus.lock().unwrap();
    let current = f.current?;
    f.groups
        .iter()
        .filter_map(|(id, group)| group.entries.iter().any(|e| e.id == current).then_some(*id))
        .next()
}

pub fn clear_focus() {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    f.desired = None;
    f.current = None;
    f.active_capture = None;
    f.active_group = None;
    for cap in f.captures.values_mut() {
        cap.desired = None;
        cap.current = None;
    }
    drop(f);
    *rt.focused_owner.lock().unwrap() = None;
}

pub fn clear_focus_owner(owner_id: OwnerId) {
    let rt = Runtime::get();
    let mut focused = rt.focused_owner.lock().unwrap();
    if *focused == Some(owner_id) {
        *focused = None;
    }
}

pub(crate) fn remove_owner_focus(owner_id: OwnerId) {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    let removed_ids: Vec<_> = f
        .entries
        .iter()
        .filter(|entry| entry.owner_id == owner_id)
        .map(|entry| entry.id)
        .collect();
    f.entries.retain(|entry| entry.owner_id != owner_id);
    let focus_id = FocusId::component(owner_id);
    if f.desired == Some(focus_id) || f.desired.is_some_and(|id| removed_ids.contains(&id)) {
        f.desired = None;
    }
    if f.current == Some(focus_id) || f.current.is_some_and(|id| removed_ids.contains(&id)) {
        f.current = None;
    }
    for group in f.groups.values_mut() {
        let group_removed: Vec<_> = group
            .entries
            .iter()
            .filter(|entry| entry.owner_id == owner_id)
            .map(|entry| entry.id)
            .collect();
        group.entries.retain(|entry| entry.owner_id != owner_id);
        if group.desired == Some(focus_id)
            || group.desired.is_some_and(|id| group_removed.contains(&id))
        {
            group.desired = None;
        }
        if group.current == Some(focus_id)
            || group.current.is_some_and(|id| group_removed.contains(&id))
        {
            group.current = None;
        }
    }
    for cap in f.captures.values_mut() {
        let removed_ids: Vec<_> = cap
            .entries
            .iter()
            .filter(|entry| entry.owner_id == owner_id)
            .map(|entry| entry.id)
            .collect();
        cap.entries.retain(|entry| entry.owner_id != owner_id);
        if cap.desired == Some(focus_id) || cap.desired.is_some_and(|id| removed_ids.contains(&id))
        {
            cap.desired = None;
        }
        if cap.current == Some(focus_id) || cap.current.is_some_and(|id| removed_ids.contains(&id))
        {
            cap.current = None;
        }
    }
    drop(f);

    let mut focused = rt.focused_owner.lock().unwrap();
    if *focused == Some(owner_id) {
        *focused = None;
    }
}
