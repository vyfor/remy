use std::future::Future;
use std::hash::Hash;
use std::marker::PhantomData;
use std::time::Duration;

use crate::handle::Init;
use crate::scope::Scope;

use super::{Query, QueryOpts};

pub struct QueryInit<KeyFn, FetchFn, K, T, E> {
    key_fn: KeyFn,
    fetch: FetchFn,
    placeholder: Option<T>,
    options: QueryOpts,
    _phantom: PhantomData<(K, E)>,
}

impl<KeyFn, FetchFn, K, T, E> QueryInit<KeyFn, FetchFn, K, T, E> {
    pub fn placeholder(mut self, value: T) -> Self {
        self.placeholder = Some(value);
        self
    }

    pub fn dedupe(mut self) -> Self {
        self.options = self.options.dedupe();
        self
    }

    pub fn cache_for(mut self, ttl: Duration) -> Self {
        self.options = self.options.cache_for(ttl);
        self
    }
}

impl<KeyFn, FetchFn, Fut, K, T, E> Init<Query<K, T, E>> for QueryInit<KeyFn, FetchFn, K, T, E>
where
    KeyFn: Fn() -> K + Send + Sync + 'static,
    FetchFn: Fn(K) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + 'static,
{
    fn install(self, handle: &'static Query<K, T, E>, _cx: Scope) {
        handle.install(self.key_fn, self.fetch, self.placeholder, self.options);
    }
}

pub fn query<KeyFn, FetchFn, Fut, K, T, E>(
    key_fn: KeyFn,
    fetch: FetchFn,
) -> QueryInit<KeyFn, FetchFn, K, T, E>
where
    KeyFn: Fn() -> K + Send + Sync + 'static,
    FetchFn: Fn(K) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    QueryInit {
        key_fn,
        fetch,
        placeholder: None,
        options: QueryOpts::default(),
        _phantom: PhantomData,
    }
}
