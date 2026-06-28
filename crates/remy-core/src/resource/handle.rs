use std::future::Future;
use std::sync::{Arc, OnceLock};

use crate::load::Load;
use crate::runtime;
use crate::state::SlotId;

use super::{ResourceInner, ResourceOpts, retry_task_id};

pub struct Resource<T: 'static, E: 'static = String> {
    inner: OnceLock<ResourceInner<T, E>>,
}

impl<T: 'static, E: 'static> Resource<T, E> {
    pub const fn uninit() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }
}

impl<T: Send + Sync + Clone + 'static, E: Send + Sync + Clone + 'static> Resource<T, E> {
    pub fn install<F, Fut>(&self, source: F, placeholder: Option<T>)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        self.install_with(source, placeholder, ResourceOpts::default());
    }

    pub fn install_with<F, Fut>(&self, source: F, placeholder: Option<T>, options: ResourceOpts)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        let inner = ResourceInner::allocate(source, placeholder, options);
        if self.inner.set(inner).is_err() {
            panic!("resource installed twice");
        }
    }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Resource<T, E> {
    fn inner_ref(&self) -> &ResourceInner<T, E> {
        self.inner
            .get()
            .expect("resource read before init")
    }

    pub fn latest(&self) -> Option<&T> {
        let inner = self.inner_ref();
        runtime::track_read(inner.data_slot);
        let slot: &Option<T> = runtime::read_current(inner.data_slot);
        slot.as_ref().or(inner.placeholder.as_ref())
    }

    pub fn value(&self) -> Option<&T> {
        if !self.has_value() {
            return None;
        }
        let inner = self.inner_ref();
        runtime::track_read(inner.data_slot);
        runtime::read_current::<Option<T>>(inner.data_slot).as_ref()
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
        let fetch_id = inner.fetch_id;
        let executor = &runtime::Runtime::get().executor;
        executor.cancel(fetch_id);
        executor.cancel(retry_task_id(fetch_id));
        runtime::write_wake(inner.loading_slot, false);
        runtime::write_wake(inner.stale_slot, false);
        let next = if self.has_value() {
            Load::Success
        } else {
            Load::Initial
        };
        runtime::write_wake(inner.status_slot, next);
    }

    pub fn reset(&self) {
        let inner = self.inner_ref();
        self.cancel();
        runtime::write_wake(inner.data_slot, None::<T>);
        runtime::write_wake(inner.error_slot, None::<E>);
        runtime::write_wake(inner.loading_slot, false);
        runtime::write_wake(inner.stale_slot, false);
        runtime::write_wake(inner.status_slot, Load::Initial);
        runtime::write_wake(inner.has_value_slot, false);
    }
}

impl<T: Send + Sync + Clone + 'static, E: Send + Sync + 'static> Resource<T, E> {
    pub fn mutate(&self, f: impl FnOnce(&mut T)) {
        let inner = self.inner_ref();
        let Some(mut owned) = runtime::read_current::<Option<T>>(inner.data_slot).clone() else {
            return;
        };

        let snapshot = owned.clone();
        inner.shadow.store(Some(Arc::new(snapshot)));

        f(&mut owned);

        runtime::write_wake(inner.data_slot, Some(owned));
    }

    pub fn rollback(&self) {
        let inner = self.inner_ref();
        if let Some(shadow_arc) = inner.shadow.swap(None) {
            let shadow_data = Arc::try_unwrap(shadow_arc).unwrap_or_else(|arc| (*arc).clone());
            runtime::write_wake(inner.data_slot, Some(shadow_data));
        }
    }
}
