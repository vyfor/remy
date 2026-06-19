use std::sync::Arc;
use std::time::Duration;

use dashmap::mapref::one::RefMut;

use crate::keyboard::{ChordPolicy, Flow, IntoBind, IntoFlow, Keys};
use crate::runtime::Runtime;
use crate::tracking::OwnerId;

pub struct StaticKeys {
    entry: RefMut<'static, OwnerId, Keys>,
}

impl StaticKeys {
    pub(crate) fn new(owner_id: OwnerId) -> Self {
        let rt = Runtime::get();
        rt.static_seen.lock().unwrap().insert(owner_id);
        Self {
            entry: rt.static_view_keys.entry(owner_id).or_default(),
        }
    }

    pub fn on_press<K, F, R>(&mut self, key: K, action: F) -> &mut Self
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        let binding = key.into_key_binding();
        if !self.entry.has_press(&binding) {
            self.entry.insert_press_once(
                binding,
                Arc::new(move || action().into_key_result()),
            );
        }
        self
    }

    pub fn on_release<K, F, R>(&mut self, key: K, action: F) -> &mut Self
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        let binding = key.into_key_binding();
        if let Some(k) = binding.first() {
            if !self.entry.has_release(k) {
                self.entry
                    .insert_release_once(k, Arc::new(move || action().into_key_result()));
            }
        }
        self
    }

    pub fn on_repeat<K, F, R>(&mut self, key: K, action: F) -> &mut Self
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        let binding = key.into_key_binding();
        if let Some(k) = binding.first() {
            if !self.entry.has_repeat(k) {
                self.entry
                    .insert_repeat_once(k, Arc::new(move || action().into_key_result()));
            }
        }
        self
    }

    pub fn on_press_any<I, K, F, R>(&mut self, keys: I, action: F) -> &mut Self
    where
        I: IntoIterator<Item = K>,
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        let action: Arc<dyn Fn() -> Flow + Send + Sync> =
            Arc::new(move || action().into_key_result());
        for key in keys {
            let act = Arc::clone(&action);
            let binding = key.into_key_binding();
            if !self.entry.has_press(&binding) {
                self.entry.insert_press_once(binding, act);
            }
        }
        self
    }

    pub fn on_release_any<I, K, F, R>(&mut self, keys: I, action: F) -> &mut Self
    where
        I: IntoIterator<Item = K>,
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        let action: Arc<dyn Fn() -> Flow + Send + Sync> =
            Arc::new(move || action().into_key_result());
        for key in keys {
            let act = Arc::clone(&action);
            let binding = key.into_key_binding();
            if let Some(k) = binding.first() {
                if !self.entry.has_release(k) {
                    self.entry.insert_release_once(k, act);
                }
            }
        }
        self
    }

    pub fn on_repeat_any<I, K, F, R>(&mut self, keys: I, action: F) -> &mut Self
    where
        I: IntoIterator<Item = K>,
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        let action: Arc<dyn Fn() -> Flow + Send + Sync> =
            Arc::new(move || action().into_key_result());
        for key in keys {
            let act = Arc::clone(&action);
            let binding = key.into_key_binding();
            if let Some(k) = binding.first() {
                if !self.entry.has_repeat(k) {
                    self.entry.insert_repeat_once(k, act);
                }
            }
        }
        self
    }

    pub fn chord_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.entry.chord_timeout(timeout);
        self
    }

    pub fn clear_chord_timeout(&mut self) -> &mut Self {
        self.entry.clear_chord_timeout();
        self
    }

    pub fn chord_policy(&mut self, policy: ChordPolicy) -> &mut Self {
        self.entry.chord_policy(policy);
        self
    }

    pub fn prefix_timeout(&mut self, prefix: impl IntoBind, timeout: Duration) -> &mut Self {
        self.entry.prefix_timeout(prefix, timeout);
        self
    }

    pub fn prefix_policy(&mut self, prefix: impl IntoBind, policy: ChordPolicy) -> &mut Self {
        self.entry.prefix_policy(prefix, policy);
        self
    }
}

pub struct LiveKeys {
    owner_id: OwnerId,
    keys: Keys,
}

impl LiveKeys {
    pub(crate) fn new(owner_id: OwnerId) -> Self {
        Self {
            owner_id,
            keys: Keys::new(),
        }
    }

    pub fn on_press<K, F, R>(&mut self, key: K, action: F) -> &mut Self
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        let action_arc: Arc<dyn Fn() -> Flow + Send + Sync> =
            Arc::new(move || action().into_key_result());
        let binding = key.into_key_binding();
        if binding.len() == 1 {
            self.keys.on_press_inner_arc(&binding, action_arc);
        } else if !binding.is_empty() {
            self.keys.on_chord_press_inner_arc(binding, action_arc);
        }
        self
    }

    pub fn on_release<K, F, R>(&mut self, key: K, action: F) -> &mut Self
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        let action_arc: Arc<dyn Fn() -> Flow + Send + Sync> =
            Arc::new(move || action().into_key_result());
        let binding = key.into_key_binding();
        if let Some(k) = binding.first() {
            self.keys.on_release_inner(k, action_arc);
        }
        self
    }

    pub fn on_repeat<K, F, R>(&mut self, key: K, action: F) -> &mut Self
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        let action_arc: Arc<dyn Fn() -> Flow + Send + Sync> =
            Arc::new(move || action().into_key_result());
        let binding = key.into_key_binding();
        if let Some(k) = binding.first() {
            self.keys.on_repeat_inner(k, action_arc);
        }
        self
    }

    pub fn chord_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.keys.chord_timeout(timeout);
        self
    }

    pub fn chord_policy(&mut self, policy: ChordPolicy) -> &mut Self {
        self.keys.chord_policy(policy);
        self
    }
}

impl Drop for LiveKeys {
    fn drop(&mut self) {
        let rt = Runtime::get();
        let mut buf = rt.live_view_key_buf.lock().unwrap();
        let entry = buf.entry(self.owner_id).or_default();
        merge_keys(entry, &self.keys);
    }
}

fn merge_keys(dest: &mut Keys, src: &Keys) {
    let src_single = src.single_binding_keys();
    let src_chord = src.chord_binding_keys();
    let src_release = src.release_binding_keys();
    let src_repeat = src.repeat_binding_keys();

    for (key, action) in src_single {
        dest.insert_press_single(key, Arc::clone(&action));
    }
    for (chord, action) in src_chord {
        dest.insert_press_chord(chord, Arc::clone(&action));
    }
    for (key, action) in src_release {
        dest.insert_release(key, Arc::clone(&action));
    }
    for (key, action) in src_repeat {
        dest.insert_repeat(key, Arc::clone(&action));
    }
}