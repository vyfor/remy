use std::cell::Cell;

use crate::effect::EffectId;

mod effect;

pub use effect::{EffectGuard, current_effect, effect_context};

pub type OwnerId = u32;

thread_local! {
    pub(crate) static ACTIVE_EFFECT: Cell<Option<EffectId>> = const { Cell::new(None) };
    pub(crate) static ACTIVE_OWNER: Cell<Option<OwnerId>> = const { Cell::new(None) };
}
