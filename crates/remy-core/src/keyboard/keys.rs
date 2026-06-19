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
pub struct Keys {
    single_bindings: HashMap<Key, Action>,
    chord_bindings: HashMap<Chord, Action>,
    chord_prefixes: HashSet<Chord>,
    prefix_options: HashMap<Chord, Prefix>,
    chord_timeout: Option<Duration>,
    chord_policy: ChordPolicy,
    release_bindings: HashMap<Key, Action>,
    repeat_bindings: HashMap<Key, Action>,
}

impl Keys {
    pub fn new() -> Self {
        Self::default()
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
            self.release_bindings
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
            self.repeat_bindings
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
                self.release_bindings.insert(k, Arc::clone(&action));
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
                self.repeat_bindings.insert(k, Arc::clone(&action));
            }
        }
        self
    }

    pub fn chord_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.chord_timeout = Some(timeout);
        self
    }

    pub fn clear_chord_timeout(&mut self) -> &mut Self {
        self.chord_timeout = None;
        self
    }

    pub fn chord_policy(&mut self, policy: ChordPolicy) -> &mut Self {
        self.chord_policy = policy;
        self
    }

    pub fn prefix_timeout(&mut self, prefix: impl IntoBind, timeout: Duration) -> &mut Self {
        let prefix = prefix.into_key_binding();
        assert!(!prefix.is_empty(), "key chord prefix cannot be empty");
        self.prefix_options.entry(prefix).or_default().timeout = Some(timeout);
        self
    }

    pub fn prefix_policy(&mut self, prefix: impl IntoBind, policy: ChordPolicy) -> &mut Self {
        let prefix = prefix.into_key_binding();
        assert!(!prefix.is_empty(), "key chord prefix cannot be empty");
        self.prefix_options.entry(prefix).or_default().policy = Some(policy);
        self
    }

    pub fn dispatch(&self, key: Key) -> Option<Flow> {
        self.single_bindings.get(&key).map(|action| action())
    }

    pub fn dispatch_release(&self, key: Key) -> Option<Flow> {
        self.release_bindings.get(&key).map(|action| action())
    }

    pub fn dispatch_repeat(&self, key: Key) -> Option<Flow> {
        self.repeat_bindings.get(&key).map(|action| action())
    }

    pub fn dispatch_chord(&self, keys: &Chord) -> Option<Flow> {
        self.chord_bindings.get(keys).map(|action| action())
    }

    pub fn has_chords(&self) -> bool {
        !self.chord_bindings.is_empty()
    }

    pub fn is_chord_prefix(&self, prefix: &Chord) -> bool {
        self.chord_prefixes.contains(prefix)
    }

    pub fn timeout_for_prefix(&self, prefix: &Chord) -> Option<Duration> {
        self.prefix_options
            .get(prefix)
            .and_then(|options| options.timeout)
            .or(self.chord_timeout)
    }

    pub fn policy_for_prefix(&self, prefix: &Chord) -> ChordPolicy {
        self.prefix_options
            .get(prefix)
            .and_then(|options| options.policy)
            .unwrap_or(self.chord_policy)
    }

    pub fn chord_completions(&self, prefix: &Chord) -> Vec<(Chord, String)> {
        let mut completions = self
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
            .chain(self.chord_bindings.keys().cloned().map(|keys| Bind {
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
        if keys.len() == 1 {
            let key = keys.first().expect("where key?");
            self.single_bindings.insert(key, action);
        } else {
            self.chord_bindings.insert(keys, action);
            self.chord_prefixes.clear();
            for keys in self.chord_bindings.keys() {
                for len in 1..keys.len() {
                    self.chord_prefixes.insert(keys.prefix(len));
                }
            }
        }
    }
}
