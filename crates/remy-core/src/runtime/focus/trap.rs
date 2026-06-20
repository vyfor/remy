use crate::runtime::Runtime;

pub fn trap_active() -> bool {
    let rt = Runtime::get();
    let focused = *rt.focused_owner.lock().unwrap();
    let f = rt.focus.lock().unwrap();
    let Some(trap_id) = f.active_trap else {
        return false;
    };
    let Some(focused) = focused else {
        return false;
    };
    f.trap_entries
        .get(&trap_id)
        .is_some_and(|entries| entries.iter().any(|entry| entry.owner_id == focused))
}

pub fn push_trap(trap_id: &'static str) {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    f.trap_stack.push(trap_id);
    f.active_trap = Some(trap_id);
    f.trap_entries.entry(trap_id).or_default();
}

pub fn pop_trap() {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    f.trap_stack.pop();
    f.active_trap = f.trap_stack.last().copied();
}

pub(crate) fn active_trap_id() -> Option<&'static str> {
    Runtime::get().focus.lock().unwrap().active_trap
}

pub(crate) fn current_frame_trap_id() -> Option<&'static str> {
    Runtime::get()
        .focus
        .lock()
        .unwrap()
        .trap_stack
        .last()
        .copied()
}

pub(crate) fn active_trap_has(owner_id: crate::tracking::OwnerId) -> bool {
    let rt = Runtime::get();
    let f = rt.focus.lock().unwrap();
    let Some(trap_id) = f.active_trap else {
        return false;
    };
    f.trap_entries
        .get(&trap_id)
        .is_some_and(|entries| entries.iter().any(|entry| entry.owner_id == owner_id))
}
