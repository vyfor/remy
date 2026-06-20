use crate::runtime::Runtime;

use super::FocusId;
use super::state::FocusEntry;

pub fn focus_next() -> bool {
    move_focus(1)
}

pub fn focus_prev() -> bool {
    move_focus(-1)
}

fn move_focus(delta: isize) -> bool {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();

    if let Some(trap_id) = f.active_trap.or_else(|| f.trap_stack.last().copied()) {
        let entries = f
            .trap_entries
            .get(&trap_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        if entries.is_empty() {
            return false;
        }
        let (_, target) = step_in(entries, f.desired.or(f.current), delta, true);
        f.desired = Some(target.id);
        f.current = Some(target.id);
        drop(f);
        *rt.focused_owner.lock().unwrap() = Some(target.owner_id);
        rt.dirty_notify.notify_one();
        return true;
    }

    let current_group = f.active_group.or_else(|| group_of(&f));
    if let Some(group_id) = current_group {
        let wrap = f
            .static_groups
            .get(&group_id)
            .map(|g| g.wrap)
            .unwrap_or(true);
        let entries = f
            .group_entries
            .get(&group_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        if !entries.is_empty() {
            let (idx, target) = step_in(entries, f.desired.or(f.current), delta, wrap);
            if idx.is_some() {
                f.desired = Some(target.id);
                f.current = Some(target.id);
                drop(f);
                *rt.focused_owner.lock().unwrap() = Some(target.owner_id);
                rt.dirty_notify.notify_one();
                return true;
            }
        }
        f.active_group = None;
    }

    if f.focus_order.is_empty() {
        return false;
    }
    let (_, target) = step_in(&f.focus_order, f.desired.or(f.current), delta, true);

    if f.static_groups.contains_key(&target.id) || f.group_entries.contains_key(&target.id) {
        f.active_group = Some(target.id);
        let first_entry = f
            .group_entries
            .get(&target.id)
            .and_then(|e| e.first())
            .copied();

        if let Some(first) = first_entry {
            f.desired = Some(first.id);
            f.current = Some(first.id);
            drop(f);
            *rt.focused_owner.lock().unwrap() = Some(first.owner_id);
        } else {
            f.desired = Some(target.id);
            f.current = Some(target.id);
            drop(f);
            *rt.focused_owner.lock().unwrap() = Some(target.owner_id);
        }
    } else {
        f.desired = Some(target.id);
        f.current = Some(target.id);
        drop(f);
        *rt.focused_owner.lock().unwrap() = Some(target.owner_id);
    }

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
    f.group_entries
        .iter()
        .find_map(|(id, entries)| entries.iter().any(|e| e.id == current).then_some(*id))
        .or_else(|| {
            f.static_groups
                .iter()
                .find_map(|(id, group)| group.members.contains(&current).then_some(*id))
        })
}
