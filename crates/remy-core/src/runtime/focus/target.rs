use std::sync::Arc;

use crate::focus_builder::FocusEventKind;
use crate::runtime::Runtime;
use crate::tracking::OwnerId;

use super::FocusId;
use super::state::{FocusEntry, StaticFocusEvents, StaticGroup};

pub fn focus_owner(owner_id: OwnerId) {
    let rt = Runtime::get();
    let focus_id = FocusId::component(owner_id);
    let mut f = rt.focus.lock().unwrap();
    if let Some(trap_id) = f.trap_stack.last().copied() {
        let entries = f.trap_entries.entry(trap_id).or_default();
        if !entries.iter().any(|e| e.id == focus_id) {
            entries.push(FocusEntry { id: focus_id, owner_id });
        }
        f.active_trap = Some(trap_id);
    }
    f.desired = Some(focus_id);
    f.current = Some(focus_id);
    drop(f);
    *rt.focused_owner.lock().unwrap() = Some(owner_id);
}

pub fn present_focus(owner_id: OwnerId) {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    let focus_id = FocusId::component(owner_id);

    f.presented.insert(owner_id);

    if !f.focus_order.iter().any(|e| e.id == focus_id) {
        f.focus_order.push(FocusEntry { id: focus_id, owner_id });
    }

    if let Some(group_id) = f.group_stack.last().copied() {
        let entries = f.group_entries.entry(group_id).or_default();
        if !entries.iter().any(|e| e.id == focus_id) {
            entries.push(FocusEntry { id: focus_id, owner_id });
        }
    }

    if let Some(trap_id) = f.trap_stack.last().copied() {
        let entries = f.trap_entries.entry(trap_id).or_default();
        if !entries.iter().any(|e| e.id == focus_id) {
            entries.push(FocusEntry { id: focus_id, owner_id });
        }
    }

    if f.current.is_none() {
        f.current = Some(focus_id);
        f.desired = Some(focus_id);
    }
}

pub fn add_focus_event(
    focus_id: FocusId,
    owner_id: OwnerId,
    kind: FocusEventKind,
    callback: impl Fn() + Send + Sync + 'static,
) {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    let events = f.static_events.entry(focus_id).or_insert(StaticFocusEvents {
        owner_id,
        on_focus: None,
        on_blur: None,
    });
    match kind {
        FocusEventKind::Focus => events.on_focus = Some(Arc::new(callback)),
        FocusEventKind::Blur => events.on_blur = Some(Arc::new(callback)),
    }
}

pub fn add_static_group_member(
    group_id: FocusId,
    owner_id: OwnerId,
    member_id: FocusId,
    wrap: bool,
) {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    let group = f.static_groups.entry(group_id).or_insert(StaticGroup {
        owner_id,
        members: Vec::new(),
        wrap,
    });
    group.wrap = wrap;
    if !group.members.contains(&member_id) {
        group.members.push(member_id);
    }
}

pub fn push_group(group_id: FocusId) {
    let rt = Runtime::get();
    rt.focus.lock().unwrap().group_stack.push(group_id);
}

pub fn pop_group() {
    let rt = Runtime::get();
    rt.focus.lock().unwrap().group_stack.pop();
}

pub fn current_focus_id() -> Option<FocusId> {
    let rt = Runtime::get();
    let f = rt.focus.lock().unwrap();
    if let Some(trap_id) = f.active_trap
        && let Some(entries) = f.trap_entries.get(trap_id)
    {
        if f.current.is_some() && entries.iter().any(|e| Some(e.id) == f.current) {
            return f.current;
        }
    }
    f.current
}

pub fn is_focus_id(focus_id: FocusId) -> bool {
    current_focus_id() == Some(focus_id)
}

pub fn focus_id(focus_id: FocusId) -> bool {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();

    let found = f.focus_order.iter().find(|e| e.id == focus_id)
        .or_else(|| f.group_entries.values().flatten().find(|e| e.id == focus_id))
        .or_else(|| f.trap_entries.values().flatten().find(|e| e.id == focus_id));

    if let Some(entry) = found.copied() {
        f.desired = Some(focus_id);
        f.current = Some(focus_id);
        drop(f);
        *rt.focused_owner.lock().unwrap() = Some(entry.owner_id);
        rt.dirty_notify.notify_one();
        return true;
    }

    f.desired = Some(focus_id);
    false
}

pub fn get_focused_owner() -> Option<OwnerId> {
    *Runtime::get().focused_owner.lock().unwrap()
}

pub fn clear_focus() {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    f.desired = None;
    f.current = None;
    f.active_trap = None;
    f.active_group = None;
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
    let focus_id = FocusId::component(owner_id);

    f.focus_order.retain(|e| e.owner_id != owner_id);
    f.static_events.retain(|_, e| e.owner_id != owner_id);
    f.static_groups.retain(|_, g| g.owner_id != owner_id);

    for group in f.static_groups.values_mut() {
        group.members.retain(|m| *m != focus_id);
    }
    for entries in f.group_entries.values_mut() {
        entries.retain(|e| e.owner_id != owner_id);
    }
    for entries in f.trap_entries.values_mut() {
        entries.retain(|e| e.owner_id != owner_id);
    }

    if f.desired == Some(focus_id) {
        f.desired = None;
    }
    if f.current == Some(focus_id) {
        f.current = None;
    }

    drop(f);

    let mut focused = rt.focused_owner.lock().unwrap();
    if *focused == Some(owner_id) {
        *focused = None;
    }
}
