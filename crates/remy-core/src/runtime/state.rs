use std::collections::HashSet;
use std::sync::OnceLock;

use crate::bus::{Commit, Op};
use crate::state::{SlotId, Slots};

use super::Runtime;

pub fn allocate_slot<T: Send + Sync + 'static>(slot_id: SlotId, initial: T) {
    Runtime::get().state.allocate(slot_id, initial);
}

pub fn next_slot_id() -> SlotId {
    crate::state::next_slot_id()
}

pub fn track_read(slot_id: SlotId) {
    if let Some(_) = crate::tracking::current_effect() {
        let rt = Runtime::get();
        rt.effects.track_read(slot_id);
        return;
    }
    if crate::tracking::is_render_tracking() {
        crate::tracking::record_render_read(slot_id);
    }
}

pub fn read_current<T: 'static>(slot_id: SlotId) -> &'static T {
    static STATE: OnceLock<&'static Slots> = OnceLock::new();
    let state = STATE.get_or_init(|| &Runtime::get().state);
    // this is fine
    unsafe { std::mem::transmute(state.read_current::<T>(slot_id)) }
}

pub fn write_wake<T: Send + Sync + 'static>(slot_id: SlotId, value: T) {
    let rt = Runtime::get();
    rt.commits.push(slot_id, value);
    rt.dirty_notify.notify_one();
}

pub fn update_wake<T, F>(slot_id: SlotId, f: F)
where
    T: Send + Sync + Clone + 'static,
    F: FnOnce(&mut T) + Send + 'static,
{
    let rt = Runtime::get();
    rt.commits.push_update::<T, F>(slot_id, f);
    rt.dirty_notify.notify_one();
}

pub fn commit_transaction(commits: Vec<Commit>) {
    if commits.is_empty() {
        return;
    }
    let rt = Runtime::get();
    rt.commits.extend(commits);
    rt.dirty_notify.notify_one();
}

pub fn apply_commits() -> Vec<SlotId> {
    let rt = Runtime::get();
    let commits = rt.commits.drain();

    for commit in commits {
        match commit.op {
            Op::Set(value) => rt.state.write_pending_raw(commit.slot_id, value),
            Op::Update(update) => update(&rt.state, commit.slot_id),
        }
    }

    let dirty_slots = rt.state.commit_all();
    rt.effects.run_slots(&dirty_slots);
    dirty_slots
}

pub fn flush_render_reads() {
    let reads = crate::tracking::drain_render_reads();
    let slots: HashSet<SlotId> = reads.into_iter().collect();
    *Runtime::get().rendered_slots.lock().unwrap() = slots;
}

pub fn should_render(dirty_slots: &[SlotId]) -> bool {
    let rt = Runtime::get();
    let rendered = rt.rendered_slots.lock().unwrap();
    if rendered.is_empty() {
        return true;
    }
    dirty_slots.iter().any(|s| rendered.contains(s))
}
