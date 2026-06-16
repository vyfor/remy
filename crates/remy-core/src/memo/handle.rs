use std::sync::OnceLock;

use crate::effect::EffectId;
use crate::runtime;
use crate::state::SlotId;

pub struct Memo<T: 'static> {
    inner: OnceLock<MemoInner<T>>,
}

pub struct MemoInner<T: 'static> {
    pub slot_id: SlotId,
    pub effect_id: EffectId,
    pub _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static> Memo<T> {
    pub const fn uninit() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }

    pub fn inner_ref(&self) -> &MemoInner<T> {
        self.inner.get().expect("memo access before install()")
    }

    pub fn id(&self) -> SlotId {
        self.inner_ref().slot_id
    }
}

impl<T: Send + Sync + Clone + PartialEq + 'static> Memo<T> {
    pub fn install(&self, derive: impl Fn() -> T + Send + Sync + 'static) {
        let slot_id = runtime::next_slot_id();

        let id_cell = std::sync::Arc::new(OnceLock::<EffectId>::new());
        let id_cell_for_cb = std::sync::Arc::clone(&id_cell);

        let derive = std::sync::Arc::new(derive);
        let derive_for_cb = std::sync::Arc::clone(&derive);

        let effect_id = runtime::register_effect(move || {
            let my_id = *id_cell_for_cb
                .get()
                .expect("memo effect ran before its id was stored");
            let _guard = crate::tracking::effect_context(my_id);
            let new_val = derive_for_cb();
            let current: &T = runtime::read_current(slot_id);
            if *current != new_val {
                runtime::write_wake(slot_id, new_val);
            }
        });

        id_cell.set(effect_id).expect("memo effect id already set");

        let first_value = {
            let _guard = crate::tracking::effect_context(effect_id);
            derive()
        };
        runtime::allocate_slot(slot_id, first_value);

        if self
            .inner
            .set(MemoInner::<T> {
                slot_id,
                effect_id,
                _phantom: std::marker::PhantomData,
            })
            .is_err()
        {
            panic!("Memo::install called twice on the same slot");
        }
    }
}

impl<T: Send + Sync + 'static> std::ops::Deref for Memo<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let inner = self.inner_ref();
        runtime::track_read(inner.slot_id);
        runtime::read_current::<T>(inner.slot_id)
    }
}

unsafe impl<T: 'static> Send for Memo<T> {}
unsafe impl<T: 'static> Sync for Memo<T> {}
