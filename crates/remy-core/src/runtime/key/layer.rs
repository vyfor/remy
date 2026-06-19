use std::collections::HashMap;
use std::sync::Arc;

use crate::keyboard::{Chord, Flow, Keys};
use crate::runtime::{FocusId, Runtime};
use crate::tracking::OwnerId;

use super::chord;

type Action = Arc<dyn Fn() -> Flow + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordOrigin {
    Focus(FocusKey),
    View(ViewKey),
    Layer(LayerId),
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewKey {
    pub(crate) owner_id: OwnerId,
    index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusKey {
    pub(crate) owner_id: OwnerId,
    pub(crate) focus_id: FocusId,
    index: u32,
}

#[derive(Clone)]
pub(crate) struct LayerEntry {
    pub(crate) id: LayerId,
    pub(crate) keys: Keys,
}

#[derive(Clone)]
pub(crate) struct ViewKeys {
    pub(crate) id: ViewKey,
    pub(crate) keys: Keys,
    pub(crate) capture: Option<&'static str>,
}

#[derive(Clone)]
pub(crate) struct FocusKeys {
    pub(crate) id: FocusKey,
    pub(crate) keys: Keys,
    pub(crate) capture: Option<&'static str>,
}

pub struct LayerHandle {
    id: LayerId,
}

impl LayerHandle {
    pub fn id(&self) -> LayerId {
        self.id
    }
}

impl Drop for LayerHandle {
    fn drop(&mut self) {
        remove_layer(self.id);
    }
}

pub fn begin_keys() {
    let rt = Runtime::get();
    rt.view_keys.lock().unwrap().clear();
    rt.view_counts.lock().unwrap().clear();
    rt.focus_keys.lock().unwrap().clear();
    rt.focus_counts.lock().unwrap().clear();
    rt.live_view_key_buf.lock().unwrap().clear();
    rt.live_focus_key_buf.lock().unwrap().clear();
    rt.static_view_seen.lock().unwrap().clear();
    rt.static_focus_seen.lock().unwrap().clear();
}

pub fn finish_keys() {
    let rt = Runtime::get();

    let live_view: HashMap<OwnerId, Keys> =
        std::mem::take(&mut *rt.live_view_key_buf.lock().unwrap());
    for (owner_id, keys) in live_view {
        let index = next_view_index(rt, owner_id);
        rt.view_keys.lock().unwrap().push(ViewKeys {
            id: ViewKey { owner_id, index },
            keys,
            capture: crate::runtime::current_frame_capture_id(),
        });
    }

    let live_focus: HashMap<(OwnerId, FocusId), Keys> =
        std::mem::take(&mut *rt.live_focus_key_buf.lock().unwrap());
    for ((owner_id, focus_id), keys) in live_focus {
        let index = next_focus_index(rt, owner_id, focus_id);
        rt.focus_keys.lock().unwrap().push(FocusKeys {
            id: FocusKey { owner_id, focus_id, index },
            keys,
            capture: crate::runtime::current_frame_capture_id(),
        });
    }

    let view_seen = std::mem::take(&mut *rt.static_view_seen.lock().unwrap());
    rt.static_view_keys
        .lock()
        .unwrap()
        .retain(|owner_id, _| view_seen.contains(owner_id));

    let focus_seen = std::mem::take(&mut *rt.static_focus_seen.lock().unwrap());
    rt.static_focus_keys
        .lock()
        .unwrap()
        .retain(|(owner_id, _), _| focus_seen.contains(owner_id));
}

pub fn add_static_view_key_press(
    owner_id: OwnerId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    add_static_view_key_press_arc(owner_id, binding, Arc::new(action));
}

pub fn add_static_view_key_press_arc(owner_id: OwnerId, binding: Chord, action: Action) {
    let rt = Runtime::get();
    rt.static_view_seen.lock().unwrap().insert(owner_id);
    rt.static_view_keys
        .lock()
        .unwrap()
        .entry(owner_id)
        .or_default()
        .insert_press_once(binding, action);
}

pub fn add_static_view_key_release(
    owner_id: OwnerId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    let rt = Runtime::get();
    rt.static_view_seen.lock().unwrap().insert(owner_id);
    if let Some(key) = binding.first() {
        rt.static_view_keys
            .lock()
            .unwrap()
            .entry(owner_id)
            .or_default()
            .insert_release_once(key, Arc::new(action));
    }
}

pub fn add_static_view_key_repeat(
    owner_id: OwnerId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    let rt = Runtime::get();
    rt.static_view_seen.lock().unwrap().insert(owner_id);
    if let Some(key) = binding.first() {
        rt.static_view_keys
            .lock()
            .unwrap()
            .entry(owner_id)
            .or_default()
            .insert_repeat_once(key, Arc::new(action));
    }
}

pub fn add_static_focus_key_press(
    owner_id: OwnerId,
    focus_id: FocusId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    let rt = Runtime::get();
    rt.static_focus_seen.lock().unwrap().insert(owner_id);
    rt.static_focus_keys
        .lock()
        .unwrap()
        .entry((owner_id, focus_id))
        .or_default()
        .insert_press_once(binding, Arc::new(action));
}

pub fn add_static_focus_key_release(
    owner_id: OwnerId,
    focus_id: FocusId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    let rt = Runtime::get();
    rt.static_focus_seen.lock().unwrap().insert(owner_id);
    if let Some(key) = binding.first() {
        rt.static_focus_keys
            .lock()
            .unwrap()
            .entry((owner_id, focus_id))
            .or_default()
            .insert_release_once(key, Arc::new(action));
    }
}

pub fn add_static_focus_key_repeat(
    owner_id: OwnerId,
    focus_id: FocusId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    let rt = Runtime::get();
    rt.static_focus_seen.lock().unwrap().insert(owner_id);
    if let Some(key) = binding.first() {
        rt.static_focus_keys
            .lock()
            .unwrap()
            .entry((owner_id, focus_id))
            .or_default()
            .insert_repeat_once(key, Arc::new(action));
    }
}

pub fn add_live_view_key_press(
    owner_id: OwnerId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    add_live_view_key_press_arc(owner_id, binding, Arc::new(action));
}

pub fn add_live_view_key_press_arc(owner_id: OwnerId, binding: Chord, action: Action) {
    Runtime::get()
        .live_view_key_buf
        .lock()
        .unwrap()
        .entry(owner_id)
        .or_default()
        .insert_press(binding, action);
}

pub fn add_live_view_key_release(
    owner_id: OwnerId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    if let Some(key) = binding.first() {
        Runtime::get()
            .live_view_key_buf
            .lock()
            .unwrap()
            .entry(owner_id)
            .or_default()
            .insert_release(key, Arc::new(action));
    }
}

pub fn add_live_view_key_repeat(
    owner_id: OwnerId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    if let Some(key) = binding.first() {
        Runtime::get()
            .live_view_key_buf
            .lock()
            .unwrap()
            .entry(owner_id)
            .or_default()
            .insert_repeat(key, Arc::new(action));
    }
}

pub fn add_live_focus_key_press(
    owner_id: OwnerId,
    focus_id: FocusId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    Runtime::get()
        .live_focus_key_buf
        .lock()
        .unwrap()
        .entry((owner_id, focus_id))
        .or_default()
        .insert_press(binding, Arc::new(action));
}

pub fn add_live_focus_key_release(
    owner_id: OwnerId,
    focus_id: FocusId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    if let Some(key) = binding.first() {
        Runtime::get()
            .live_focus_key_buf
            .lock()
            .unwrap()
            .entry((owner_id, focus_id))
            .or_default()
            .insert_release(key, Arc::new(action));
    }
}

pub fn add_live_focus_key_repeat(
    owner_id: OwnerId,
    focus_id: FocusId,
    binding: Chord,
    action: impl Fn() -> Flow + Send + Sync + 'static,
) {
    if let Some(key) = binding.first() {
        Runtime::get()
            .live_focus_key_buf
            .lock()
            .unwrap()
            .entry((owner_id, focus_id))
            .or_default()
            .insert_repeat(key, Arc::new(action));
    }
}

pub fn cancel_owner(owner_id: OwnerId) {
    chord::cancel_owner(owner_id);
}

pub fn set_global_keys(keys: Keys) {
    *Runtime::get().global_keys.lock().unwrap() = keys;
}

pub fn add_layer(keys: Keys) -> LayerHandle {
    let rt = Runtime::get();
    let id = LayerId(
        rt.next_layer
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    );
    rt.layers.lock().unwrap().push(LayerEntry { id, keys });
    LayerHandle { id }
}

fn remove_layer(id: LayerId) {
    let rt = Runtime::get();
    rt.layers.lock().unwrap().retain(|entry| entry.id != id);
    if chord::clear_layer(id) {
        rt.dirty_notify.notify_one();
    }
}

pub fn remove_static_keys(owner_id: OwnerId) {
    let rt = Runtime::get();
    rt.static_view_keys.lock().unwrap().remove(&owner_id);
    rt.static_focus_keys
        .lock()
        .unwrap()
        .retain(|(oid, _), _| *oid != owner_id);
}

pub fn layers() -> Vec<(LayerId, Keys)> {
    Runtime::get()
        .layers
        .lock()
        .unwrap()
        .iter()
        .rev()
        .map(|entry| (entry.id, entry.keys.clone()))
        .collect()
}

pub fn view_keys() -> Vec<(ViewKey, Keys)> {
    let cap = crate::runtime::active_capture_id();
    let rt = Runtime::get();

    let mut result: Vec<_> = rt
        .view_keys
        .lock()
        .unwrap()
        .iter()
        .rev()
        .filter(|entry| allows(entry.capture, cap))
        .map(|entry| (entry.id, entry.keys.clone()))
        .collect();

    for (owner_id, keys) in rt.static_view_keys.lock().unwrap().iter() {
        result.push((ViewKey { owner_id: *owner_id, index: 0 }, keys.clone()));
    }

    result
}

pub fn focus_keys() -> Vec<(FocusKey, Keys)> {
    let Some(focus_id) = crate::runtime::current_focus_id() else {
        return Vec::new();
    };
    let cap = crate::runtime::active_capture_id();
    let rt = Runtime::get();

    let mut result: Vec<_> = rt
        .focus_keys
        .lock()
        .unwrap()
        .iter()
        .rev()
        .filter(|entry| entry.id.focus_id == focus_id && allows(entry.capture, cap))
        .map(|entry| (entry.id, entry.keys.clone()))
        .collect();

    let focused_owner = *rt.focused_owner.lock().unwrap();
    if let Some(owner_id) = focused_owner {
        if let Some(keys) = rt.static_focus_keys.lock().unwrap().get(&(owner_id, focus_id)) {
            result.push((FocusKey { owner_id, focus_id, index: 0 }, keys.clone()));
        }
    }

    result
}

pub fn keys_for(origin: ChordOrigin) -> Option<Keys> {
    let rt = Runtime::get();
    match origin {
        ChordOrigin::Focus(id) => rt
            .focus_keys
            .lock()
            .unwrap()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.keys.clone())
            .or_else(|| {
                rt.static_focus_keys
                    .lock()
                    .unwrap()
                    .get(&(id.owner_id, id.focus_id))
                    .cloned()
            }),
        ChordOrigin::View(id) => rt
            .view_keys
            .lock()
            .unwrap()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.keys.clone())
            .or_else(|| {
                rt.static_view_keys
                    .lock()
                    .unwrap()
                    .get(&id.owner_id)
                    .cloned()
            }),
        ChordOrigin::Layer(id) => rt
            .layers
            .lock()
            .unwrap()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.keys.clone()),
        ChordOrigin::Global => Some(rt.global_keys.lock().unwrap().clone()),
    }
}

pub(super) fn layer_exists(id: LayerId) -> bool {
    Runtime::get()
        .layers
        .lock()
        .unwrap()
        .iter()
        .any(|entry| entry.id == id)
}

pub(super) fn view_exists(id: ViewKey) -> bool {
    let cap = crate::runtime::active_capture_id();
    let rt = Runtime::get();
    rt.view_keys
        .lock()
        .unwrap()
        .iter()
        .any(|entry| entry.id == id && allows(entry.capture, cap))
        || rt.static_view_keys.lock().unwrap().contains_key(&id.owner_id)
}

pub(super) fn focus_exists(id: FocusKey) -> bool {
    let cap = crate::runtime::active_capture_id();
    Runtime::get()
        .focus_keys
        .lock()
        .unwrap()
        .iter()
        .any(|entry| entry.id == id && allows(entry.capture, cap))
}

fn next_view_index(rt: &Runtime, owner_id: OwnerId) -> u32 {
    let mut counts = rt.view_counts.lock().unwrap();
    let index = counts.entry(owner_id).or_default();
    let current = *index;
    *index += 1;
    current
}

fn next_focus_index(rt: &Runtime, owner_id: OwnerId, focus_id: FocusId) -> u32 {
    let mut counts = rt.focus_counts.lock().unwrap();
    let index = counts.entry((owner_id, focus_id)).or_default();
    let current = *index;
    *index += 1;
    current
}

fn allows(entry_capture: Option<&'static str>, active_capture: Option<&'static str>) -> bool {
    match active_capture {
        Some(active) => entry_capture == Some(active),
        None => true,
    }
}
