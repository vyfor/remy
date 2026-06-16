use std::future::Future;
use std::hash::Hash;
use std::marker::PhantomData;
use std::time::Duration;

use crate::proxy::Init;
use crate::scope::Scope;

use super::{Query, QueryOpts};

pub struct QueryInit<KeyFn, FetchFn, K, T, E> {
    key_fn: KeyFn,
    fetch: FetchFn,
    options: QueryOpts,
    _phantom: PhantomData<(K, T, E)>,
}

impl<KeyFn, FetchFn, K, T, E> QueryInit<KeyFn, FetchFn, K, T, E> {
    pub fn initial(self, initial: T) -> QuerySeed<KeyFn, FetchFn, K, T, E> {
        QuerySeed {
            key_fn: self.key_fn,
            fetch: self.fetch,
            initial,
            options: self.options,
            _phantom: PhantomData,
        }
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

pub struct QuerySeed<KeyFn, FetchFn, K, T, E> {
    key_fn: KeyFn,
    fetch: FetchFn,
    initial: T,
    options: QueryOpts,
    _phantom: PhantomData<(K, T, E)>,
}

impl<KeyFn, FetchFn, K, T, E> QuerySeed<KeyFn, FetchFn, K, T, E> {
    pub fn dedupe(mut self) -> Self {
        self.options = self.options.dedupe();
        self
    }

    pub fn cache_for(mut self, ttl: Duration) -> Self {
        self.options = self.options.cache_for(ttl);
        self
    }
}

impl<KeyFn, FetchFn, Fut, K, T, E> Init<Query<K, T, E>> for QuerySeed<KeyFn, FetchFn, K, T, E>
where
    KeyFn: Fn() -> K + Send + Sync + 'static,
    FetchFn: Fn(K) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + 'static,
{
    fn install(self, handle: &'static Query<K, T, E>, _cx: Scope) {
        handle.install(self.key_fn, self.fetch, self.initial, self.options);
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
        options: QueryOpts::default(),
        _phantom: PhantomData,
    }
}
