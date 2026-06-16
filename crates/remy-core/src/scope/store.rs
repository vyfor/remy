use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use crate::owner::Owner;

use super::Scope;

#[derive(Clone)]
pub struct StoreCx {
    scope: Scope,
    owner: Arc<Owner>,
}

impl StoreCx {
    pub fn new(scope: Scope, owner: Arc<Owner>) -> Self {
        Self { scope, owner }
    }

    pub fn global<T: Send + Sync + Clone + 'static>(&self) -> T {
        self.scope.global::<T>()
    }

    pub fn try_global<T: Send + Sync + Clone + 'static>(&self) -> Option<T> {
        self.scope.try_global::<T>()
    }

    pub fn global_arc<T: Send + Sync + 'static>(&self) -> Arc<T> {
        self.scope.global_arc::<T>()
    }

    pub fn try_global_arc<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.scope.try_global_arc::<T>()
    }

    pub fn spawn<F: Future<Output = ()> + Send + 'static>(&self, fut: F) {
        self.owner.spawn(fut);
    }

    pub fn interval<F>(&self, period: Duration, tick: F)
    where
        F: FnMut() + Send + 'static,
    {
        self.owner.interval(period, tick);
    }

    pub fn timeout<F>(&self, delay: Duration, action: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.owner.timeout(delay, action);
    }

    pub fn effect(&self, callback: impl Fn() + Send + Sync + 'static) -> crate::effect::EffectId {
        self.owner.register_effect(callback)
    }

    pub fn detached(&self) -> Scope {
        self.scope
    }
}
