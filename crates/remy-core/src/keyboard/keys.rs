use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use super::{Bind, BindKind, Chord, ChordPolicy, Flow, IntoBind, IntoFlow, Key};

type Action = Arc<dyn Fn() -> Flow + Send + Sync>;

#[derive(Debug, Clone, Copy, Default)]
struct Prefix {
    timeout: Option<Duration>,
    policy: Option<ChordPolicy>,
}

#[derive(Clone, Default)]
pub(crate) struct KeysInner {
    single_bindings: HashMap<Key, Action>,
    chord_bindings: HashMap<Chord, Action>,
    chord_prefixes: HashSet<Chord>,
    prefix_options: HashMap<Chord, Prefix>,
    chord_timeout: Option<Duration>,
    chord_policy: ChordPolicy,
    release_bindings: HashMap<Key, Action>,
    repeat_bindings: HashMap<Key, Action>,
}

#[derive(Clone, Default)]
pub struct Keys {
    inner: Arc<KeysInner>,
}

impl Keys {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn inner_mut(&mut self) -> &mut KeysInner {
        Arc::make_mut(&mut self.inner)
    }

    pub(crate) fn insert_press_once(&mut self, binding: Chord, action: Action) {
        let inner = Arc::make_mut(&mut self.inner);
        if binding.len() == 1 {
            let key = binding.first().unwrap();
            inner.single_bindings.entry(key).or_insert(action);
        } else if !binding.is_empty() {
            inner.chord_bindings.entry(binding).or_insert(action);
            rebuild_chord_prefixes(inner);
        }
    }

    pub(crate) fn insert_release_once(&mut self, key: Key, action: Action) {
        Arc::make_mut(&mut self.inner)
            .release_bindings
            .entry(key)
            .or_insert(action);
    }

    pub(crate) fn insert_repeat_once(&mut self, key: Key, action: Action) {
        Arc::make_mut(&mut self.inner)
            .repeat_bindings
            .entry(key)
            .or_insert(action);
    }

    pub(crate) fn insert_press(&mut self, binding: Chord, action: Action) {
        let inner = Arc::make_mut(&mut self.inner);
        if binding.len() == 1 {
            let key = binding.first().unwrap();
            inner.single_bindings.insert(key, action);
        } else if !binding.is_empty() {
            inner.chord_bindings.insert(binding, action);
            rebuild_chord_prefixes(inner);
        }
    }

    pub(crate) fn insert_release(&mut self, key: Key, action: Action) {
        Arc::make_mut(&mut self.inner)
            .release_bindings
            .insert(key, action);
    }

    pub(crate) fn insert_repeat(&mut self, key: Key, action: Action) {
        Arc::make_mut(&mut self.inner)
            .repeat_bindings
            .insert(key, action);
    }

    pub(crate) fn has_press(&self, binding: &Chord) -> bool {
        if binding.len() == 1 {
            let key = binding.first().unwrap();
            self.inner.single_bindings.contains_key(&key)
        } else {
            self.inner.chord_bindings.contains_key(binding)
        }
    }

    pub(crate) fn has_release(&self, key: Key) -> bool {
        self.inner.release_bindings.contains_key(&key)
    }

    pub(crate) fn has_repeat(&self, key: Key) -> bool {
        self.inner.repeat_bindings.contains_key(&key)
    }

    pub(crate) fn insert_press_single(&mut self, key: Key, action: Action) {
        Arc::make_mut(&mut self.inner)
            .single_bindings
            .insert(key, action);
    }

    pub(crate) fn insert_press_chord(&mut self, chord: Chord, action: Action) {
        let inner = Arc::make_mut(&mut self.inner);
        inner.chord_bindings.insert(chord, action);
        rebuild_chord_prefixes(inner);
    }

    pub(crate) fn on_press_inner_arc(&mut self, binding: &Chord, action: Action) {
        if binding.len() == 1 {
            let key = binding.first().unwrap();
            self.insert_press_single(key, action);
        } else if !binding.is_empty() {
            self.insert_press_chord(binding.clone(), action);
        }
    }

    pub(crate) fn on_chord_press_inner_arc(&mut self, binding: Chord, action: Action) {
        self.insert_press_chord(binding, action);
    }

    pub(crate) fn on_release_inner(&mut self, key: Key, action: Action) {
        Arc::make_mut(&mut self.inner)
            .release_bindings
            .insert(key, action);
    }

    pub(crate) fn on_repeat_inner(&mut self, key: Key, action: Action) {
        Arc::make_mut(&mut self.inner)
            .repeat_bindings
            .insert(key, action);
    }

    pub(crate) fn single_binding_keys(&self) -> Vec<(Key, Action)> {
        self.inner
            .single_bindings
            .iter()
            .map(|(k, v)| (*k, Arc::clone(v)))
            .collect()
    }

    pub(crate) fn chord_binding_keys(&self) -> Vec<(Chord, Action)> {
        self.inner
            .chord_bindings
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect()
    }

    pub(crate) fn release_binding_keys(&self) -> Vec<(Key, Action)> {
        self.inner
            .release_bindings
            .iter()
            .map(|(k, v)| (*k, Arc::clone(v)))
            .collect()
    }

    pub(crate) fn repeat_binding_keys(&self) -> Vec<(Key, Action)> {
        self.inner
            .repeat_bindings
            .iter()
            .map(|(k, v)| (*k, Arc::clone(v)))
            .collect()
    }

    pub fn on_press<K, F, R>(&mut self, key: K, action: F) -> &mut Self
    where
        K: IntoBind,
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        let action: Action = Arc::new(move || action().into_key_result());
        self.bind_action(key.into_key_binding(), action);
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
            self.inner_mut()
                .release_bindings
                .insert(k, Arc::new(move || action().into_key_result()));
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
            self.inner_mut()
                .repeat_bindings
                .insert(k, Arc::new(move || action().into_key_result()));
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
        let action: Action = Arc::new(move || action().into_key_result());
        for key in keys {
            self.bind_action(key.into_key_binding(), Arc::clone(&action));
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
        let action: Action = Arc::new(move || action().into_key_result());
        for key in keys {
            let binding = key.into_key_binding();
            if let Some(k) = binding.first() {
                self.inner_mut()
                    .release_bindings
                    .insert(k, Arc::clone(&action));
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
        let action: Action = Arc::new(move || action().into_key_result());
        for key in keys {
            let binding = key.into_key_binding();
            if let Some(k) = binding.first() {
                self.inner_mut()
                    .repeat_bindings
                    .insert(k, Arc::clone(&action));
            }
        }
        self
    }

    pub fn chord_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.inner_mut().chord_timeout = Some(timeout);
        self
    }

    pub fn clear_chord_timeout(&mut self) -> &mut Self {
        self.inner_mut().chord_timeout = None;
        self
    }

    pub fn chord_policy(&mut self, policy: ChordPolicy) -> &mut Self {
        self.inner_mut().chord_policy = policy;
        self
    }

    pub fn prefix_timeout(&mut self, prefix: impl IntoBind, timeout: Duration) -> &mut Self {
        let prefix = prefix.into_key_binding();
        assert!(!prefix.is_empty(), "key chord prefix cannot be empty");
        self.inner_mut()
            .prefix_options
            .entry(prefix)
            .or_default()
            .timeout = Some(timeout);
        self
    }

    pub fn prefix_policy(&mut self, prefix: impl IntoBind, policy: ChordPolicy) -> &mut Self {
        let prefix = prefix.into_key_binding();
        assert!(!prefix.is_empty(), "key chord prefix cannot be empty");
        self.inner_mut()
            .prefix_options
            .entry(prefix)
            .or_default()
            .policy = Some(policy);
        self
    }

    pub fn dispatch(&self, key: Key) -> Option<Flow> {
        self.inner.single_bindings.get(&key).map(|action| action())
    }

    pub fn dispatch_release(&self, key: Key) -> Option<Flow> {
        self.inner.release_bindings.get(&key).map(|action| action())
    }

    pub fn dispatch_repeat(&self, key: Key) -> Option<Flow> {
        self.inner.repeat_bindings.get(&key).map(|action| action())
    }

    pub fn dispatch_chord(&self, keys: &Chord) -> Option<Flow> {
        self.inner.chord_bindings.get(keys).map(|action| action())
    }

    pub fn has_chords(&self) -> bool {
        !self.inner.chord_bindings.is_empty()
    }

    pub fn is_chord_prefix(&self, prefix: &Chord) -> bool {
        self.inner.chord_prefixes.contains(prefix)
    }

    pub fn timeout_for_prefix(&self, prefix: &Chord) -> Option<Duration> {
        self.inner
            .prefix_options
            .get(prefix)
            .and_then(|options| options.timeout)
            .or(self.inner.chord_timeout)
    }

    pub fn policy_for_prefix(&self, prefix: &Chord) -> ChordPolicy {
        self.inner
            .prefix_options
            .get(prefix)
            .and_then(|options| options.policy)
            .unwrap_or(self.inner.chord_policy)
    }

    pub fn chord_completions(&self, prefix: &Chord) -> Vec<(Chord, String)> {
        let mut completions = self
            .inner
            .chord_bindings
            .keys()
            .filter(|keys| keys.starts_with(prefix) && keys.len() > prefix.len())
            .map(|keys| (keys.clone(), keys.label()))
            .collect::<Vec<_>>();
        completions.sort_by(|(_, left), (_, right)| left.cmp(right));
        completions
    }

    pub fn labels(&self) -> Vec<String> {
        self.descriptions()
            .into_iter()
            .map(|info| info.label)
            .collect()
    }

    pub fn descriptions(&self) -> Vec<Bind> {
        let mut descriptions = self
            .inner
            .single_bindings
            .keys()
            .copied()
            .map(|key| {
                let keys = Chord::from(key);
                Bind {
                    label: keys.label(),
                    keys,
                    kind: BindKind::Single,
                }
            })
            .chain(self.inner.chord_bindings.keys().cloned().map(|keys| Bind {
                label: keys.label(),
                keys,
                kind: BindKind::Chord,
            }))
            .collect::<Vec<_>>();
        descriptions.sort_by(|left, right| left.label.cmp(&right.label));
        descriptions
    }

    fn bind_action(&mut self, keys: Chord, action: Action) {
        assert!(!keys.is_empty(), "key binding cannot be empty");
        let inner = self.inner_mut();
        if keys.len() == 1 {
            let key = keys.first().expect("where key?");
            inner.single_bindings.insert(key, action);
        } else {
            inner.chord_bindings.insert(keys, action);
            rebuild_chord_prefixes(inner);
        }
    }
}

fn rebuild_chord_prefixes(inner: &mut KeysInner) {
    inner.chord_prefixes.clear();
    for keys in inner.chord_bindings.keys() {
        for len in 1..keys.len() {
            inner.chord_prefixes.insert(keys.prefix(len));
        }
    }
}
