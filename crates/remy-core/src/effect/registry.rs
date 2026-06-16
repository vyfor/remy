use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::SlotId;

use super::{Effect, EffectId};

pub struct Effects {
    pub(crate) effects: Mutex<HashMap<EffectId, Arc<Effect>>>,
    slot_effects: Mutex<HashMap<SlotId, Vec<EffectId>>>,
    next_id: AtomicU32,
}

impl Default for Effects {
    fn default() -> Self {
        Self::new()
    }
}

impl Effects {
    pub fn new() -> Self {
        Self {
            effects: Mutex::new(HashMap::new()),
            slot_effects: Mutex::new(HashMap::new()),
            next_id: AtomicU32::new(0),
        }
    }

    pub fn register(&self, callback: impl Fn() + Send + Sync + 'static) -> EffectId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let entry = Arc::new(Effect {
            id,
            callback: Box::new(callback),
            tracked_slots: Mutex::new(HashSet::new()),
        });
        self.effects.lock().unwrap().insert(id, entry);
        id
    }

    pub fn dispose(&self, id: EffectId) {
        let mut effects = self.effects.lock().unwrap();
        if let Some(effect) = effects.remove(&id) {
            let tracked = effect.tracked_slots.lock().unwrap();
            let mut slot_effs = self.slot_effects.lock().unwrap();
            for slot_id in tracked.iter() {
                if let Some(list) = slot_effs.get_mut(slot_id) {
                    list.retain(|&eid| eid != id);
                }
            }
        }
    }

    pub fn track_read(&self, slot_id: SlotId) {
        if let Some(effect_id) = crate::tracking::current_effect() {
            let effects = self.effects.lock().unwrap();
            if let Some(effect) = effects.get(&effect_id)
                && effect.tracked_slots.lock().unwrap().insert(slot_id)
            {
                let mut slot_effs = self.slot_effects.lock().unwrap();
                let list = slot_effs.entry(slot_id).or_default();
                if !list.contains(&effect_id) {
                    list.push(effect_id);
                }
            }
        }
    }

    pub fn run_slots(&self, dirty_slots: &[SlotId]) {
        if dirty_slots.is_empty() {
            return;
        }
        let to_run: Vec<EffectId> = {
            let slot_effs = self.slot_effects.lock().unwrap();
            let mut collected = Vec::new();
            for &slot_id in dirty_slots {
                if let Some(list) = slot_effs.get(&slot_id) {
                    for &eid in list {
                        if !collected.contains(&eid) {
                            collected.push(eid);
                        }
                    }
                }
            }
            collected
        };

        if crate::runtime::is_batching() {
            crate::runtime::BATCH_QUEUE.with(|q| q.borrow_mut().extend(to_run));
        } else {
            for id in to_run {
                self.run_effect_by_id(id);
            }
        }
    }

    pub fn run_ids(&self, ids: &[EffectId]) {
        for &id in ids {
            self.run_effect_by_id(id);
        }
    }

    pub fn run_effect_by_id(&self, id: EffectId) {
        let arc = {
            let effects = self.effects.lock().unwrap();
            effects.get(&id).cloned()
        };
        let Some(arc) = arc else { return };

        {
            let old_slots = {
                let mut tracked = arc.tracked_slots.lock().unwrap();
                let slots: Vec<SlotId> = tracked.drain().collect();
                slots
            };
            let mut slot_effs = self.slot_effects.lock().unwrap();
            for slot_id in old_slots {
                if let Some(list) = slot_effs.get_mut(&slot_id) {
                    list.retain(|&eid| eid != id);
                }
            }
        }

        let _guard = crate::tracking::effect_context(id);
        (arc.callback)();
    }
}
