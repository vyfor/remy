use crate::runtime::Runtime;

use super::FocusId;
use super::state::FocusEntry;

pub fn focus_next() -> bool {
    move_focus(1)
}

pub fn focus_prev() -> bool {
    move_focus(-1)
}

pub fn focus_next_group() -> bool {
    move_group(1)
}

pub fn focus_prev_group() -> bool {
    move_group(-1)
}

pub fn focus_enter_group() -> bool {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();

    if f.current
        .is_some_and(|id| f.groups.contains_key(&id) || group_of(&f).is_some())
        && group_of(&f).is_some()
    {
        return false;
    }

    let Some(current) = f.current else {
        return false;
    };
    let Some(group) = f.groups.get(&current) else {
        return false;
    };
    if group.entries.is_empty() {
        return false;
    }

    let first = group.entries[0];
    f.active_group = Some(current);
    f.desired = Some(first.id);
    f.current = Some(first.id);
    *rt.focused_owner.lock().unwrap() = Some(first.owner_id);
    rt.dirty_notify.notify_one();
    true
}

pub fn focus_leave_group() -> bool {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();

    let Some(group_id) = group_of(&f) else {
        return false;
    };

    f.active_group = None;
    f.desired = Some(group_id);
    f.current = Some(group_id);
    if let Some(entry) = f.entries.iter().find(|e| e.id == group_id).copied() {
        *rt.focused_owner.lock().unwrap() = Some(entry.owner_id);
    }
    rt.dirty_notify.notify_one();
    true
}

fn move_focus(delta: isize) -> bool {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();

    if let Some(cap_id) = f.active_capture {
        let Some(cap) = f.captures.get_mut(cap_id) else {
            return false;
        };
        if cap.entries.is_empty() {
            return false;
        }

        let (_idx, target) = step_in(
            cap.entries.as_slice(),
            cap.desired.or(cap.current),
            delta,
            true,
        );
        cap.desired = Some(target.id);
        cap.current = Some(target.id);
        *rt.focused_owner.lock().unwrap() = Some(target.owner_id);
        rt.dirty_notify.notify_one();
        return true;
    }

    if let Some(group_id) = group_of(&f) {
        let wrap = f.groups.get(&group_id).map(|g| g.wrap).unwrap_or(true);
        let group = f.groups.get_mut(&group_id).unwrap();
        if group.entries.is_empty() {
            return false;
        }
        let (idx, target) = step_in(
            group.entries.as_slice(),
            group.desired.or(group.current),
            delta,
            wrap,
        );
        if !wrap && idx.is_none() {
            return false;
        }
        group.desired = Some(target.id);
        group.current = Some(target.id);
        f.desired = Some(target.id);
        f.current = Some(target.id);
        *rt.focused_owner.lock().unwrap() = Some(target.owner_id);
        rt.dirty_notify.notify_one();
        return true;
    }

    if f.entries.is_empty() {
        return false;
    }
    let (_idx, target) = step_in(f.entries.as_slice(), f.desired.or(f.current), delta, true);
    f.desired = Some(target.id);
    f.current = Some(target.id);
    *rt.focused_owner.lock().unwrap() = Some(target.owner_id);
    rt.dirty_notify.notify_one();
    true
}

fn move_group(delta: isize) -> bool {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();

    let group_headers: Vec<FocusId> = f
        .entries
        .iter()
        .filter(|entry| f.groups.contains_key(&entry.id))
        .map(|entry| entry.id)
        .collect();

    if group_headers.is_empty() {
        return false;
    }

    let current_group = f.active_group.or_else(|| group_of(&f));

    let current_idx: isize = current_group
        .and_then(|id| group_headers.iter().position(|h| *h == id))
        .map(|i| i as isize)
        .unwrap_or(-1);
    let len = group_headers.len() as isize;
    let next_idx = if current_idx < 0 {
        if delta > 0 { 0 } else { len - 1 }
    } else {
        let wrapped = (current_idx + delta).rem_euclid(len) as usize;
        wrapped as isize
    };
    let next_group_id = group_headers[next_idx as usize];

    f.active_group = Some(next_group_id);
    let group = f.groups.get(&next_group_id).unwrap();
    let target = group.entries.first().copied().unwrap_or_else(|| {
        f.entries
            .iter()
            .find(|e| e.id == next_group_id)
            .copied()
            .expect("group header MUST exist in f.entries")
    });

    f.desired = Some(target.id);
    f.current = Some(target.id);
    if let Some(group) = f.groups.get_mut(&next_group_id) {
        group.desired = Some(target.id);
        group.current = Some(target.id);
    }
    *rt.focused_owner.lock().unwrap() = Some(target.owner_id);
    rt.dirty_notify.notify_one();
    true
}

fn step_in(
    entries: &[FocusEntry],
    current: Option<FocusId>,
    delta: isize,
    wrap: bool,
) -> (Option<usize>, FocusEntry) {
    debug_assert!(!entries.is_empty());
    let current_idx = current
        .and_then(|id| entries.iter().position(|e| e.id == id))
        .unwrap_or(0);
    let len = entries.len() as isize;
    let raw = current_idx as isize + delta;
    if wrap {
        let next = raw.rem_euclid(len) as usize;
        (Some(next), entries[next])
    } else if raw < 0 || raw >= len {
        (None, entries[current_idx])
    } else {
        let next = raw as usize;
        (Some(next), entries[next])
    }
}

fn group_of(f: &super::state::FocusState) -> Option<FocusId> {
    let current = f.current?;
    f.groups
        .iter()
        .find_map(|(id, group)| group.entries.iter().any(|e| e.id == current).then_some(*id))
}
