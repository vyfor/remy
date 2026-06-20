use std::time::{Duration, Instant};

use crate::keyboard::{Chord, ChordPolicy, Key};
use crate::tracking::OwnerId;

use crate::runtime::{Runtime, active_trap_has, active_trap_id, current_focus_id};

use super::layer::{self, ChordOrigin, LayerId, keys_for};

#[derive(Debug, Clone)]
pub struct PendingChord {
    pub keys: Chord,
    pub origin: ChordOrigin,
    pub policy: ChordPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpiredChord {
    pub first_key: Key,
    pub origin: ChordOrigin,
}

#[derive(Debug, Clone)]
pub(crate) struct ChordState {
    pending: Chord,
    origin: Option<ChordOrigin>,
    started_at: Option<Instant>,
    timeout: Option<Duration>,
    policy: ChordPolicy,
}

impl Default for ChordState {
    fn default() -> Self {
        Self {
            pending: Chord::new(),
            origin: None,
            started_at: None,
            timeout: None,
            policy: ChordPolicy::Discard,
        }
    }
}

impl ChordState {
    fn is_pending(&self) -> bool {
        self.origin.is_some() && !self.pending.is_empty()
    }

    fn clear(&mut self) {
        self.pending = Chord::new();
        self.origin = None;
        self.started_at = None;
        self.timeout = None;
        self.policy = ChordPolicy::Discard;
    }

    fn deadline(&self) -> Option<Instant> {
        let started_at = self.started_at?;
        let timeout = self.timeout?;
        Some(started_at + timeout)
    }
}

pub(super) fn clear_layer(id: LayerId) -> bool {
    let mut state = Runtime::get().chord.lock().unwrap();
    let changed = matches!(state.origin, Some(ChordOrigin::Layer(origin)) if origin == id);
    if changed {
        state.clear();
    }
    changed
}

pub fn pending_chord() -> Option<PendingChord> {
    let state = Runtime::get().chord.lock().unwrap();
    Some(PendingChord {
        keys: state.pending.clone(),
        origin: state.origin?,
        policy: state.policy,
    })
    .filter(|pending| !pending.keys.is_empty())
}

pub fn start_chord(
    origin: ChordOrigin,
    pending: Chord,
    timeout: Option<Duration>,
    policy: ChordPolicy,
) {
    let rt = Runtime::get();
    {
        let mut state = rt.chord.lock().unwrap();
        state.pending = pending;
        state.origin = Some(origin);
        state.started_at = Some(Instant::now());
        state.timeout = timeout;
        state.policy = policy;
    }
    rt.dirty_notify.notify_one();
}

pub fn update_chord(pending: Chord, timeout: Option<Duration>, policy: ChordPolicy) {
    let rt = Runtime::get();
    {
        let mut state = rt.chord.lock().unwrap();
        if state.is_pending() {
            state.pending = pending;
            state.started_at = Some(Instant::now());
            state.timeout = timeout;
            state.policy = policy;
        }
    }
    rt.dirty_notify.notify_one();
}

pub fn reset_chord() {
    let rt = Runtime::get();
    let changed = {
        let mut state = rt.chord.lock().unwrap();
        let changed = state.is_pending();
        state.clear();
        changed
    };
    if changed {
        rt.dirty_notify.notify_one();
    }
}

pub fn cancel_owner(owner_id: OwnerId) {
    let rt = Runtime::get();
    let changed = {
        let mut state = rt.chord.lock().unwrap();
        let changed = match state.origin {
            Some(ChordOrigin::Focus(origin)) => origin.owner_id == owner_id,
            Some(ChordOrigin::View(origin)) => origin.owner_id == owner_id,
            _ => false,
        };
        if changed {
            state.clear();
        }
        changed
    };
    if changed {
        rt.dirty_notify.notify_one();
    }
}

pub fn chord_deadline() -> Option<Instant> {
    Runtime::get().chord.lock().unwrap().deadline()
}

pub fn take_expired_chord() -> Option<ExpiredChord> {
    let rt = Runtime::get();
    let expired = {
        let mut state = rt.chord.lock().unwrap();
        let deadline = state.deadline()?;
        if Instant::now() < deadline {
            return None;
        }
        let first_key = state.pending.first()?;
        let origin = state.origin?;
        state.clear();
        Some(ExpiredChord { first_key, origin })
    };
    if expired.is_some() {
        rt.dirty_notify.notify_one();
    }
    expired
}

pub fn cancel_stale_chord() -> bool {
    let Some(pending) = pending_chord() else {
        return false;
    };
    if !chord_stale(pending.origin) {
        return false;
    }
    reset_chord();
    true
}

pub fn chord_stale(origin: ChordOrigin) -> bool {
    match origin {
        ChordOrigin::Global => active_trap_id().is_some(),
        ChordOrigin::Layer(layer_id) => {
            active_trap_id().is_some() || !layer::layer_exists(layer_id)
        }
        ChordOrigin::Focus(id) => {
            if current_focus_id() != Some(id.focus_id) {
                return true;
            }
            !layer::focus_exists(id)
                || (active_trap_id().is_some() && !active_trap_has(id.owner_id))
        }
        ChordOrigin::View(id) => !layer::view_exists(id),
    }
}

pub fn pending_chord_keys() -> Option<Vec<Key>> {
    pending_chord().map(|pending| pending.keys.as_slice().to_vec())
}

pub fn pending_chord_label() -> Option<String> {
    pending_chord().map(|pending| pending.keys.label())
}

pub fn chord_completions() -> Vec<(Chord, String)> {
    let Some(pending) = pending_chord() else {
        return Vec::new();
    };
    keys_for(pending.origin)
        .map(|bindings| bindings.chord_completions(&pending.keys))
        .unwrap_or_default()
}
