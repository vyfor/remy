use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::MissedTickBehavior;

use crate::owner::Owner;

mod globals;

pub use globals::Globals;

#[derive(Clone, Default)]
pub struct App {
    owner: Option<Arc<Owner>>,
}

impl App {
    pub fn new() -> Self {
        Self { owner: None }
    }

    #[doc(hidden)]
    pub fn with_owner(owner: Arc<Owner>) -> Self {
        Self { owner: Some(owner) }
    }

    pub fn global<T: Send + Sync + Clone + 'static>(&self) -> T {
        crate::runtime::get_global::<T>()
            .unwrap_or_else(|| panic!("global of type {} not provided", std::any::type_name::<T>()))
    }

    pub fn try_global<T: Send + Sync + Clone + 'static>(&self) -> Option<T> {
        crate::runtime::get_global::<T>()
    }

    pub fn global_arc<T: Send + Sync + 'static>(&self) -> Arc<T> {
        crate::runtime::get_global_arc::<T>()
            .unwrap_or_else(|| panic!("global of type {} not provided", std::any::type_name::<T>()))
    }

    pub fn try_global_arc<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        crate::runtime::get_global_arc::<T>()
    }

    pub fn spawn<F: Future<Output = ()> + Send + 'static>(&self, fut: F) {
        match &self.owner {
            Some(owner) => owner.spawn(fut),
            None => {
                tokio::spawn(fut);
            }
        }
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

    pub fn effect(&self, callback: impl Fn() + Send + Sync + 'static) -> crate::effect::EffectId {
        match &self.owner {
            Some(owner) => owner.register_effect(callback),
            None => {
                let id = crate::runtime::register_effect(callback);
                crate::runtime::run_effect_by_id(id);
                id
            }
        }
    }
}
