use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::time::MissedTickBehavior;

use crate::effect::EffectId;
use crate::keyboard::Keys;
use crate::runtime;
use crate::runtime::LayerHandle;
use crate::tracking::OwnerId;

pub struct Owner {
    inner: Arc<OwnerInner>,
}

struct OwnerInner {
    effects: Mutex<Vec<EffectId>>,
    owners: Mutex<Vec<OwnerId>>,
    layers: Mutex<Vec<LayerHandle>>,
    tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl Owner {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(OwnerInner {
                effects: Mutex::new(Vec::new()),
                owners: Mutex::new(Vec::new()),
                layers: Mutex::new(Vec::new()),
                tasks: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn register_effect(&self, callback: impl Fn() + Send + Sync + 'static) -> EffectId {
        let id = runtime::register_effect(callback);
        self.inner.effects.lock().unwrap().push(id);
        runtime::run_effect_by_id(id);
        id
    }

    pub fn spawn<F>(&self, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let handle = tokio::spawn(fut);
        self.inner.tasks.lock().unwrap().push(handle);
    }

    pub fn interval<F>(&self, period: Duration, mut tick: F)
    where
        F: FnMut() + Send + 'static,
    {
        self.spawn(async move {
            let mut interval = tokio::time::interval(period);
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                tick();
            }
        });
    }

    pub fn timeout<F>(&self, delay: Duration, action: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.spawn(async move {
            tokio::time::sleep(delay).await;
            action();
        });
    }

    pub fn track_owner(&self, id: OwnerId) {
        self.inner.owners.lock().unwrap().push(id);
    }

    pub fn register_key(&self, configure: impl FnOnce(&mut Keys)) {
        let mut bindings = Keys::new();
        configure(&mut bindings);
        let handle = runtime::add_layer(bindings);
        self.inner.layers.lock().unwrap().push(handle);
    }
}
impl Default for Owner {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        for handle in self.inner.tasks.lock().unwrap().drain(..) {
            handle.abort();
        }
        let effect_ids = std::mem::take(&mut *self.inner.effects.lock().unwrap());
        for id in effect_ids {
            runtime::dispose_effect(id);
        }
        let owner_ids = std::mem::take(&mut *self.inner.owners.lock().unwrap());
        for id in owner_ids {
            runtime::dispose_owner(id);
        }
        self.inner.layers.lock().unwrap().clear();
    }
}
