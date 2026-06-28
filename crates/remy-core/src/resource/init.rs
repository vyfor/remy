use std::future::Future;
use std::marker::PhantomData;
use std::time::Duration;

use crate::proxy::Init;
use crate::scope::Scope;

use super::{Resource, ResourceOpts, Retry};

pub struct ResourceInit<F, T, E> {
    source: F,
    placeholder: Option<T>,
    options: ResourceOpts,
    _phantom: PhantomData<E>,
}

impl<F, T, E> ResourceInit<F, T, E> {
    pub fn placeholder(mut self, value: T) -> Self {
        self.placeholder = Some(value);
        self
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

impl<F, Fut, T, E> Init<Resource<T, E>> for ResourceInit<F, T, E>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + 'static,
{
    fn install(self, handle: &'static Resource<T, E>, _cx: Scope) {
        handle.install_with(self.source, self.placeholder, self.options);
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
        placeholder: None,
        options: ResourceOpts::default(),
        _phantom: PhantomData,
    }
}
