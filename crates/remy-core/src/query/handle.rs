use std::future::Future;
use std::hash::Hash;
use std::sync::OnceLock;

use crate::load::Load;
use crate::runtime;
use crate::state::SlotId;

use super::{QueryInner, QueryOpts};

pub struct Query<K: 'static, T: 'static, E: 'static = String> {
    inner: OnceLock<QueryInner<K, T, E>>,
}

impl<K: 'static, T: 'static, E: 'static> Query<K, T, E> {
    pub const fn uninit() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }
}

impl<K, T, E> Query<K, T, E>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + 'static,
{
    pub fn install<KeyFn, FetchFn, Fut>(
        &self,
        key_fn: KeyFn,
        fetch: FetchFn,
        initial: T,
        options: QueryOpts,
    ) where
        KeyFn: Fn() -> K + Send + Sync + 'static,
        FetchFn: Fn(K) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        let inner = QueryInner::allocate(key_fn, fetch, initial, options);
        if self.inner.set(inner).is_err() {
            panic!("Query::install called twice on the same slot");
        }
    }
}

impl<K: Send + Sync + 'static, T: Send + Sync + 'static, E: Send + Sync + 'static> Query<K, T, E> {
    fn inner_ref(&self) -> &QueryInner<K, T, E> {
        self.inner
            .get()
            .expect("query access before run() init phase")
    }

    pub fn latest(&self) -> &T {
        self.data()
    }

    pub fn data(&self) -> &T {
        let inner = self.inner_ref();
        runtime::track_read(inner.data_slot);
        let slot: &Option<T> = runtime::read_current(inner.data_slot);
        slot.as_ref().unwrap_or(&inner.initial)
    }

    pub fn value(&self) -> Option<&T> {
        if !self.has_value() {
            return None;
        }
        let inner = self.inner_ref();
        runtime::track_read(inner.data_slot);
        runtime::read_current::<Option<T>>(inner.data_slot).as_ref()
    }

    pub fn map_value<R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.value().map(f)
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(self.data())
    }

    pub fn status(&self) -> Load {
        let inner = self.inner_ref();
        runtime::track_read(inner.status_slot);
        *runtime::read_current::<Load>(inner.status_slot)
    }

    pub fn has_value(&self) -> bool {
        let inner = self.inner_ref();
        runtime::track_read(inner.has_value_slot);
        *runtime::read_current::<bool>(inner.has_value_slot)
    }

    pub fn is_initial(&self) -> bool {
        self.status().is_initial()
    }

    pub fn is_loading(&self) -> bool {
        self.status().is_loading()
    }

    pub fn is_success(&self) -> bool {
        self.status().is_success()
    }

    pub fn is_refreshing(&self) -> bool {
        self.status().is_refreshing()
    }

    pub fn is_error(&self) -> bool {
        self.status().is_error()
    }

    pub fn is_pending(&self) -> bool {
        self.status().is_pending()
    }

    pub fn is_settled(&self) -> bool {
        self.status().is_settled()
    }

    pub fn loading(&self) -> bool {
        self.is_loading()
    }

    pub fn stale(&self) -> bool {
        self.is_refreshing()
    }

    pub fn error(&self) -> Option<&E> {
        let inner = self.inner_ref();
        runtime::track_read(inner.error_slot);
        runtime::read_current::<Option<E>>(inner.error_slot).as_ref()
    }

    pub fn refetch(&self) -> SlotId {
        let id = self.inner_ref().effect_id;
        runtime::run_effect_by_id(id);
        id
    }

    pub fn id(&self) -> SlotId {
        self.inner_ref().data_slot
    }

    pub fn cancel(&self) {
        let inner = self.inner_ref();
        runtime::Runtime::get().executor.cancel(inner.fetch_id);
        *inner.in_flight.lock().unwrap() = None;
        runtime::write_wake(inner.loading_slot, false);
        runtime::write_wake(inner.stale_slot, false);
        let next = if self.has_value() {
            Load::Success
        } else {
            Load::Initial
        };
        runtime::write_wake(inner.status_slot, next);
    }
}

impl<K: Send + Sync + 'static, T: Send + Sync + Clone + 'static, E: Send + Sync + 'static>
    Query<K, T, E>
{
    pub fn value_or(&self, fallback: T) -> T {
        self.value().cloned().unwrap_or(fallback)
    }

    pub fn value_or_else(&self, fallback: impl FnOnce() -> T) -> T {
        self.value().cloned().unwrap_or_else(fallback)
    }
}

impl<K: Send + Sync + 'static, T: Send + Sync + 'static, E: Send + Sync + 'static> std::ops::Deref
    for Query<K, T, E>
{
    type Target = T;

    fn deref(&self) -> &T {
        self.data()
    }
}
