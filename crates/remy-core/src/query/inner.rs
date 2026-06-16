use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use crate::effect::EffectId;
use crate::load::Load;
use crate::runtime;
use crate::state::SlotId;

use super::QueryOpts;

static NEXT_QUERY_ID: AtomicU32 = AtomicU32::new(0x9000_0000);

pub(super) struct QueryInner<K: 'static, T: 'static, E: 'static> {
    pub(super) data_slot: SlotId,
    pub(super) loading_slot: SlotId,
    pub(super) stale_slot: SlotId,
    pub(super) error_slot: SlotId,
    pub(super) status_slot: SlotId,
    pub(super) has_value_slot: SlotId,
    pub(super) fetch_id: u32,
    pub(super) effect_id: EffectId,
    pub(super) initial: T,
    pub(super) in_flight: Arc<Mutex<Option<K>>>,
    _phantom: PhantomData<E>,
}

#[derive(Clone)]
struct CacheEntry<T> {
    data: T,
    written_at: Instant,
}

impl<K, T, E> QueryInner<K, T, E>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + 'static,
{
    pub(super) fn allocate<KeyFn, FetchFn, Fut>(
        key_fn: KeyFn,
        fetch: FetchFn,
        initial: T,
        options: QueryOpts,
    ) -> Self
    where
        KeyFn: Fn() -> K + Send + Sync + 'static,
        FetchFn: Fn(K) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
    {
        let data_slot = runtime::next_slot_id();
        let loading_slot = runtime::next_slot_id();
        let stale_slot = runtime::next_slot_id();
        let error_slot = runtime::next_slot_id();
        let status_slot = runtime::next_slot_id();
        let has_value_slot = runtime::next_slot_id();

        runtime::allocate_slot(data_slot, Some(initial.clone()));
        runtime::allocate_slot(loading_slot, false);
        runtime::allocate_slot(stale_slot, false);
        runtime::allocate_slot(error_slot, None::<E>);
        runtime::allocate_slot(status_slot, Load::Initial);
        runtime::allocate_slot(has_value_slot, false);

        let fetch_id = NEXT_QUERY_ID.fetch_add(1, Ordering::Relaxed);
        let fetched = Arc::new(AtomicBool::new(false));
        let in_flight: Arc<Mutex<Option<K>>> = Arc::new(Mutex::new(None));
        let cache: Arc<Mutex<HashMap<K, CacheEntry<T>>>> = Arc::new(Mutex::new(HashMap::new()));

        let id_cell: Arc<OnceLock<EffectId>> = Arc::new(OnceLock::new());
        let id_cell_for_cb = Arc::clone(&id_cell);
        let fetched_for_cb = Arc::clone(&fetched);
        let in_flight_for_cb = Arc::clone(&in_flight);
        let cache_for_cb = Arc::clone(&cache);

        let effect_id = runtime::register_effect(move || {
            let my_id = *id_cell_for_cb
                .get()
                .expect("effect ran before its id was registered");

            let key = {
                let _track = crate::tracking::effect_context(my_id);
                key_fn()
            };

            if let Some(ttl) = options.cache_for
                && let Some(entry) = cache_for_cb.lock().unwrap().get(&key).cloned()
                && entry.written_at.elapsed() <= ttl
            {
                runtime::write_wake(data_slot, Some(entry.data));
                runtime::write_wake(loading_slot, false);
                runtime::write_wake(stale_slot, false);
                runtime::write_wake(error_slot, None::<E>);
                runtime::write_wake(status_slot, Load::Success);
                runtime::write_wake(has_value_slot, true);
                fetched_for_cb.store(true, Ordering::Release);
                return;
            }

            if options.dedupe {
                let mut in_flight_key = in_flight_for_cb.lock().unwrap();
                if in_flight_key.as_ref() == Some(&key) {
                    return;
                }
                *in_flight_key = Some(key.clone());
            }

            if fetched_for_cb.load(Ordering::Acquire) {
                runtime::write_wake(stale_slot, true);
                runtime::write_wake(status_slot, Load::Refreshing);
            } else {
                runtime::write_wake(loading_slot, true);
                runtime::write_wake(status_slot, Load::Loading);
            }

            let gen_number = runtime::bump_resource_gen(fetch_id);
            let fut = fetch(key.clone());
            let in_flight_f = Arc::clone(&in_flight_for_cb);
            let cache_f = Arc::clone(&cache_for_cb);
            let fetched_f = Arc::clone(&fetched_for_cb);

            runtime::Runtime::get()
                .executor
                .dispatch(fetch_id, async move {
                    let result = fut.await;
                    if runtime::current_resource_gen(fetch_id) != gen_number {
                        return;
                    }

                    let mut in_flight_key = in_flight_f.lock().unwrap();
                    if in_flight_key.as_ref() == Some(&key) {
                        *in_flight_key = None;
                    }
                    drop(in_flight_key);

                    match result {
                        Ok(data) => {
                            if options.cache_for.is_some() {
                                cache_f.lock().unwrap().insert(
                                    key,
                                    CacheEntry {
                                        data: data.clone(),
                                        written_at: Instant::now(),
                                    },
                                );
                            }
                            runtime::write_wake(data_slot, Some(data));
                            runtime::write_wake(loading_slot, false);
                            runtime::write_wake(stale_slot, false);
                            runtime::write_wake(error_slot, None::<E>);
                            runtime::write_wake(status_slot, Load::Success);
                            runtime::write_wake(has_value_slot, true);
                            fetched_f.store(true, Ordering::Release);
                        }
                        Err(e) => {
                            runtime::write_wake(error_slot, Some(e));
                            runtime::write_wake(loading_slot, false);
                            runtime::write_wake(stale_slot, false);
                            runtime::write_wake(status_slot, Load::Error);
                        }
                    }
                });
        });

        id_cell.set(effect_id).expect("effect id already populated");

        Self {
            data_slot,
            loading_slot,
            stale_slot,
            error_slot,
            status_slot,
            has_value_slot,
            fetch_id,
            effect_id,
            initial,
            in_flight,
            _phantom: PhantomData,
        }
    }
}
