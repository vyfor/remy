use std::future::Future;
use std::marker::PhantomData;
use std::time::Duration;

use crate::proxy::Init;
use crate::scope::Scope;

use super::{Resource, ResourceOpts, Retry};

pub struct ResourceInit<F, T, E> {
    source: F,
    options: ResourceOpts,
    _phantom: PhantomData<(T, E)>,
}

impl<F, T, E> ResourceInit<F, T, E> {
    pub fn initial(self, initial: T) -> ResourceSeed<F, T, E> {
        ResourceSeed {
            source: self.source,
            initial,
            options: self.options,
            _phantom: PhantomData,
        }
    }

    pub fn retry(mut self, policy: Retry) -> Self {
        self.options = self.options.retry(policy);
        self
    }

    pub fn refresh_every(mut self, period: Duration) -> Self {
        self.options = self.options.refresh_every(period);
        self
    }
}

pub struct ResourceSeed<F, T, E> {
    source: F,
    initial: T,
    options: ResourceOpts,
    _phantom: PhantomData<(T, E)>,
}

impl<F, T, E> ResourceSeed<F, T, E> {
    pub fn retry(mut self, policy: Retry) -> Self {
        self.options = self.options.retry(policy);
        self
    }

    pub fn refresh_every(mut self, period: Duration) -> Self {
        self.options = self.options.refresh_every(period);
        self
    }
}

impl<F, Fut, T, E> Init<Resource<T, E>> for ResourceSeed<F, T, E>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + 'static,
{
    fn install(self, handle: &'static Resource<T, E>, _cx: Scope) {
        handle.install_with(self.source, self.initial, self.options);
    }
}

pub fn resource<F, Fut, T, E>(source: F) -> ResourceInit<F, T, E>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    ResourceInit {
        source,
        options: ResourceOpts::default(),
        _phantom: PhantomData,
    }
}
