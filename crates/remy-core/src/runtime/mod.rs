use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64};

use dashmap::DashMap;
use ratatui::layout::Rect;
use tokio::sync::Notify;

use crate::bus::{Executor, Queue};
use crate::cached::ComponentCache;
use crate::effect::Effects;
use crate::keyboard::Keys;
use crate::mouse::Regions;
use crate::scope::{Globals, Scope};
use crate::state::{SlotId, Slots};
use crate::tracking::OwnerId;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

mod batch;
mod component;
mod effect;
mod focus;
mod global;
mod key;
mod mouse;
mod overlay;
mod resource;
mod state;

pub(crate) use batch::BATCH_QUEUE;
pub use batch::{batch_enter, batch_exit, flush_batch, is_batching};
pub(crate) use component::next_id;
pub use component::{dispose_owner, register_owner, set_active_owner, spawn_owner};
pub use effect::{dispose_effect, register_effect, run_effect_by_id};
pub use focus::{
    FocusId, active_group, begin_focus_frame, capture_active, clear_focus, clear_focus_owner,
    current_focus_id, declare_focus, declare_group, declare_in_group, finish_focus_frame,
    focus_enter_group, focus_id, focus_leave_group, focus_next, focus_next_group, focus_owner,
    focus_prev, focus_prev_group, get_focused_owner, is_focus_id, set_group_wrap, with_capture,
};
pub(crate) use focus::{
    FocusState, active_capture_has, active_capture_id, current_frame_capture_id, remove_owner_focus,
};
pub use global::{dispatch_intent, get_global, get_global_arc, report_error};
pub use key::{
    ChordOrigin, ExpiredChord, FocusKey, LayerHandle, LayerId, PendingChord, ViewKey,
    add_focus_keys, add_layer, add_view_keys, begin_keys, cancel_owner, cancel_stale_chord,
    chord_completions, chord_deadline, chord_stale, focus_keys, keys_for, layers, pending_chord,
    pending_chord_keys, pending_chord_label, reset_chord, set_global_keys, start_chord,
    take_expired_chord, update_chord, view_keys,
};
pub(crate) use key::{ChordState, FocusKeys, LayerEntry, ViewKeys};
pub use mouse::{
    add_mouse_region, begin_mouse_frame, dispatch_mouse_event, finish_mouse_frame,
    is_region_hovered,
};
pub use overlay::{drain_overlays, overlay_rects, push_overlay};
pub use resource::{
    bump_resource_gen, current_resource_gen, has_resource_fetched, mark_resource_fetched,
};
pub use state::{
    allocate_slot, apply_commits, commit_transaction, flush_render_reads, next_slot_id,
    read_current, should_render, track_read, update_wake, write_wake,
};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub struct Runtime {
    pub(crate) state: Slots,
    pub(crate) effects: Effects,
    pub executor: Executor,
    pub(crate) commits: Queue,
    pub globals: Globals,
    pub(crate) dirty_notify: Notify,
    rendering: AtomicBool,
    next_owner_id: std::sync::atomic::AtomicU32,
    pub(crate) resource_gens: DashMap<u32, AtomicU64>,
    pub(crate) resource_fetched: DashMap<u32, AtomicBool>,
    pub(crate) focused_owner: Mutex<Option<OwnerId>>,
    pub(crate) focus: Mutex<FocusState>,
    pub(crate) view_keys: Mutex<Vec<ViewKeys>>,
    pub(crate) view_counts: Mutex<HashMap<OwnerId, u32>>,
    pub(crate) focus_keys: Mutex<Vec<FocusKeys>>,
    pub(crate) focus_counts: Mutex<HashMap<(OwnerId, FocusId), u32>>,
    pub(crate) global_keys: Mutex<Keys>,
    pub(crate) layers: Mutex<Vec<LayerEntry>>,
    next_layer: AtomicU64,
    pub(crate) chord: Mutex<ChordState>,
    pub(crate) mouse: Mutex<Regions>,
    rendered_slots: Mutex<HashSet<SlotId>>,
    redraw_requested: AtomicBool,
    pub(crate) component_caches: DashMap<OwnerId, ComponentCache>,
    pending_dirty: Mutex<Vec<SlotId>>,
    pub(crate) canvas: Mutex<Option<Rect>>,
}

impl Runtime {
    fn new(globals: Globals) -> Self {
        Self {
            state: Slots::new(),
            effects: Effects::new(),
            executor: Executor::new(),
            commits: Queue::new(),
            globals,
            dirty_notify: Notify::new(),
            rendering: AtomicBool::new(false),
            next_owner_id: std::sync::atomic::AtomicU32::new(next_id()),
            resource_gens: DashMap::new(),
            resource_fetched: DashMap::new(),
            focused_owner: Mutex::new(None),
            focus: Mutex::new(FocusState::default()),
            view_keys: Mutex::new(Vec::new()),
            view_counts: Mutex::new(HashMap::new()),
            focus_keys: Mutex::new(Vec::new()),
            focus_counts: Mutex::new(HashMap::new()),
            global_keys: Mutex::new(Keys::new()),
            layers: Mutex::new(Vec::new()),
            next_layer: AtomicU64::new(1),
            chord: Mutex::new(ChordState::default()),
            mouse: Mutex::new(Regions::default()),
            rendered_slots: Mutex::new(HashSet::new()),
            redraw_requested: AtomicBool::new(false),
            component_caches: DashMap::new(),
            pending_dirty: Mutex::new(Vec::new()),
            canvas: Mutex::new(None),
        }
    }

    pub fn init(globals: Globals) -> &'static Self {
        let rt = RUNTIME.get_or_init(|| Self::new(globals));
        init_stores();
        rt
    }

    pub fn get() -> &'static Self {
        let rt = RUNTIME.get_or_init(|| Self::new(Globals::new()));
        init_stores();
        rt
    }
}

fn init_stores() {
    static STORES_INITIALIZED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);
    if STORES_INITIALIZED.swap(true, std::sync::atomic::Ordering::AcqRel) {
        return;
    }

    crate::check_slot_collisions();
    let scope = Scope::new();
    for init_fn in crate::STORE_REGISTRY {
        init_fn(scope);
    }
}

pub fn dirty_notify() -> &'static Notify {
    &Runtime::get().dirty_notify
}

pub fn redraw() {
    let rt = Runtime::get();
    rt.redraw_requested
        .store(true, std::sync::atomic::Ordering::Relaxed);
    rt.dirty_notify.notify_one();
}

pub(crate) fn take_redraw() -> bool {
    Runtime::get()
        .redraw_requested
        .swap(false, std::sync::atomic::Ordering::Relaxed)
}

pub fn begin_render() {
    Runtime::get()
        .rendering
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

pub fn end_render() {
    Runtime::get()
        .rendering
        .store(false, std::sync::atomic::Ordering::Relaxed);
}

pub fn is_rendering() -> bool {
    Runtime::get()
        .rendering
        .load(std::sync::atomic::Ordering::Relaxed)
}

pub(crate) fn set_canvas(area: Rect) {
    let rt = Runtime::get();
    if let Ok(mut guard) = rt.canvas.lock() {
        *guard = Some(area);
    }
}

pub fn invalidate_all() {
    let rt = Runtime::get();
    let area = rt
        .canvas
        .lock()
        .ok()
        .and_then(|guard| *guard)
        .unwrap_or(Rect::new(0, 0, 0, 0));
    crate::tracking::mark_cleared(area);
    redraw();
}
