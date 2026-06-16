use std::future::Future;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use arc_swap::ArcSwapOption;

use crate::effect::EffectId;
use crate::load::Load;
use crate::runtime;
use crate::state::SlotId;

use super::{Refresh, ResourceOpts};

static NEXT_RESOURCE_ID: AtomicU32 = AtomicU32::new(0x8000_0000);
const RETRY_TASK_OFFSET: u32 = 0x2000_0000;
const REFRESH_TASK_OFFSET: u32 = 0x4000_0000;

pub(super) fn retry_task_id(fetch_id: u32) -> u32 {
    fetch_id ^ RETRY_TASK_OFFSET
}

pub(super) fn refresh_task_id(fetch_id: u32) -> u32 {
    fetch_id ^ REFRESH_TASK_OFFSET
}

pub(super) struct ResourceInner<T: 'static, E: 'static> {
    pub(super) data_slot: SlotId,
    pub(super) loading_slot: SlotId,
    pub(super) stale_slot: SlotId,
    pub(super) error_slot: SlotId,
    pub(super) status_slot: SlotId,
    pub(super) has_value_slot: SlotId,
    pub(super) shadow: ArcSwapOption<T>,
    pub(super) fetch_id: u32,
    pub(super) effect_id: EffectId,
    pub(super) initial: T,
    _phantom: PhantomData<E>,
}

impl<T: Send + Sync + Clone + 'static, E: Send + Sync + Clone + 'static> ResourceInner<T, E> {
    pub(super) fn allocate<F, Fut>(source: F, initial: T, options: ResourceOpts) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
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

        let fetch_id = NEXT_RESOURCE_ID.fetch_add(1, Ordering::Relaxed);

        let retry_attempts = Arc::new(AtomicU32::new(0));
        let retry_trigger = Arc::new(AtomicBool::new(false));
        let refresh_started = Arc::new(AtomicBool::new(false));
        let id_cell: Arc<OnceLock<EffectId>> = Arc::new(OnceLock::new());
        let id_cell_for_cb = Arc::clone(&id_cell);
        let retry_attempts_for_cb = Arc::clone(&retry_attempts);
        let retry_trigger_for_cb = Arc::clone(&retry_trigger);
        let refresh_started_for_cb = Arc::clone(&refresh_started);

        let effect_id = runtime::register_effect(move || {
            let my_id = *id_cell_for_cb
                .get()
                .expect("effect ran before its id was registered");
            if !retry_trigger_for_cb.swap(false, Ordering::AcqRel) {
                retry_attempts_for_cb.store(0, Ordering::Release);
                runtime::Runtime::get()
                    .executor
                    .cancel(retry_task_id(fetch_id));
            }

            if let Refresh::Every(period) = options.refresh
                && !refresh_started_for_cb.swap(true, Ordering::AcqRel)
            {
                runtime::Runtime::get()
                    .executor
                    .dispatch(refresh_task_id(fetch_id), async move {
                        let mut interval = tokio::time::interval(period);
                        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                        interval.tick().await;
                        loop {
                            interval.tick().await;
                            runtime::run_effect_by_id(my_id);
                        }
                    });
            }

            let _track = crate::tracking::effect_context(my_id);
            let fut = source();

            let has_value = *runtime::read_current::<bool>(has_value_slot);
            if has_value {
                runtime::write_wake(stale_slot, true);
                runtime::write_wake(status_slot, Load::Refreshing);
            } else {
                runtime::write_wake(loading_slot, true);
                runtime::write_wake(status_slot, Load::Loading);
            }

            let gen_number = runtime::bump_resource_gen(fetch_id);
            let retry_policy = options.retry;
            let retry_attempts_f = Arc::clone(&retry_attempts_for_cb);
            let retry_trigger_f = Arc::clone(&retry_trigger_for_cb);

            let data_slot_f = data_slot;
            let loading_slot_f = loading_slot;
            let stale_slot_f = stale_slot;
            let error_slot_f = error_slot;
            let status_slot_f = status_slot;
            let has_value_slot_f = has_value_slot;

            runtime::Runtime::get()
                .executor
                .dispatch(fetch_id, async move {
                    let result = fut.await;
                    if runtime::current_resource_gen(fetch_id) != gen_number {
                        return;
                    }
                    match result {
                        Ok(data) => {
                            runtime::write_wake(data_slot_f, Some(data));
                            runtime::write_wake(loading_slot_f, false);
                            runtime::write_wake(stale_slot_f, false);
                            runtime::write_wake(error_slot_f, None::<E>);
                            runtime::write_wake(status_slot_f, Load::Success);
                            runtime::write_wake(has_value_slot_f, true);
                            retry_attempts_f.store(0, Ordering::Release);
                            runtime::mark_resource_fetched(fetch_id);
                        }
                        Err(e) => {
                            runtime::write_wake(error_slot_f, Some(e));
                            runtime::write_wake(loading_slot_f, false);
                            runtime::write_wake(stale_slot_f, false);
                            runtime::write_wake(status_slot_f, Load::Error);
                            let retry_number = retry_attempts_f.fetch_add(1, Ordering::AcqRel) + 1;
                            if let Some(delay) = retry_policy.delay_for_retry(retry_number) {
                                runtime::Runtime::get().executor.dispatch_debounced(
                                    retry_task_id(fetch_id),
                                    delay,
                                    async move {
                                        retry_trigger_f.store(true, Ordering::Release);
                                        runtime::run_effect_by_id(my_id);
                                    },
                                );
                            }
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
            shadow: ArcSwapOption::empty(),
            fetch_id,
            effect_id,
            initial,
            _phantom: PhantomData,
        }
    }
}
