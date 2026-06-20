use crate::runtime::Runtime;

pub fn begin_focus_frame() {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();
    f.focus_order.clear();
    f.presented.clear();
    f.group_stack.clear();
    f.group_entries.clear();
    f.trap_stack.clear();
    f.trap_entries.clear();
    f.active_trap = None;
    f.active_group = None;
    f.previous = f.current;
}

pub fn finish_focus_frame() {
    let rt = Runtime::get();
    let mut f = rt.focus.lock().unwrap();

    let presented_set = f.presented.clone();
    
    f.static_events
        .retain(|_, e| presented_set.contains(&e.owner_id));
    f.static_groups
        .retain(|_, g| presented_set.contains(&g.owner_id));

    if let Some(current) = f.current {
        let still_present = f.focus_order.iter().any(|e| e.id == current)
            || f.group_entries.values().flatten().any(|e| e.id == current)
            || f.trap_entries.values().flatten().any(|e| e.id == current);
        if !still_present {
            f.current = f.focus_order.first().map(|e| e.id);
            f.desired = f.current;
        }
    }

    if f.current.is_none() && !f.focus_order.is_empty() {
        f.current = Some(f.focus_order[0].id);
        f.desired = f.current;
    }

    let focused_owner = f.current.and_then(|id| {
        f.focus_order
            .iter()
            .find(|e| e.id == id)
            .or_else(|| f.group_entries.values().flatten().find(|e| e.id == id))
            .or_else(|| f.trap_entries.values().flatten().find(|e| e.id == id))
            .map(|e| e.owner_id)
    });

    let prev = f.previous;
    let curr = f.current;

    drop(f);
    *rt.focused_owner.lock().unwrap() = focused_owner;

    if curr != prev {
        let f = rt.focus.lock().unwrap();
        if let Some(prev_id) = prev {
            if let Some(events) = f.static_events.get(&prev_id) {
                if let Some(on_blur) = &events.on_blur {
                    on_blur();
                }
            }
        }
        if let Some(curr_id) = curr {
            if let Some(events) = f.static_events.get(&curr_id) {
                if let Some(on_focus) = &events.on_focus {
                    on_focus();
                }
            }
        }
    }
}
