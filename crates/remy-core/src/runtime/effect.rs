use crate::effect::EffectId;

use super::Runtime;

pub fn register_effect(callback: impl Fn() + Send + Sync + 'static) -> EffectId {
    Runtime::get().effects.register(callback)
}

pub fn dispose_effect(effect_id: EffectId) {
    Runtime::get().effects.dispose(effect_id);
}

pub fn run_effect_by_id(effect_id: EffectId) {
    Runtime::get().effects.run_effect_by_id(effect_id)
}
