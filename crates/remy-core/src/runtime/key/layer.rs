use crate::keyboard::Keys;
use crate::runtime::{FocusId, Runtime};
use crate::tracking::OwnerId;

use super::chord;

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
}

pub fn add_view_keys(owner_id: OwnerId, keys: Keys) -> ViewKey {
    let rt = Runtime::get();
    let index = next_view_index(rt, owner_id);
    let id = ViewKey { owner_id, index };

    rt.view_keys.lock().unwrap().push(ViewKeys {
        id,
        keys,
        capture: crate::runtime::current_frame_capture_id(),
    });

    id
}

pub fn add_focus_keys(owner_id: OwnerId, focus_id: FocusId, keys: Keys) -> FocusKey {
    let rt = Runtime::get();
    let index = next_focus_index(rt, owner_id, focus_id);
    let id = FocusKey {
        owner_id,
        focus_id,
        index,
    };

    rt.focus_keys.lock().unwrap().push(FocusKeys {
        id,
        keys,
        capture: crate::runtime::current_frame_capture_id(),
    });

    id
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
    Runtime::get()
        .view_keys
        .lock()
        .unwrap()
        .iter()
        .rev()
        .filter(|entry| allows(entry.capture, cap))
        .map(|entry| (entry.id, entry.keys.clone()))
        .collect()
}

pub fn focus_keys() -> Vec<(FocusKey, Keys)> {
    let Some(focus_id) = crate::runtime::current_focus_id() else {
        return Vec::new();
    };
    let cap = crate::runtime::active_capture_id();
    Runtime::get()
        .focus_keys
        .lock()
        .unwrap()
        .iter()
        .rev()
        .filter(|entry| entry.id.focus_id == focus_id && allows(entry.capture, cap))
        .map(|entry| (entry.id, entry.keys.clone()))
        .collect()
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
            .map(|entry| entry.keys.clone()),
        ChordOrigin::View(id) => rt
            .view_keys
            .lock()
            .unwrap()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.keys.clone()),
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
    Runtime::get()
        .view_keys
        .lock()
        .unwrap()
        .iter()
        .any(|entry| entry.id == id && allows(entry.capture, cap))
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
