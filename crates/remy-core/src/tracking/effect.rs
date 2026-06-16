use crate::effect::EffectId;

use super::ACTIVE_EFFECT;

pub struct EffectGuard {
    previous: Option<EffectId>,
}

impl Drop for EffectGuard {
    fn drop(&mut self) {
        ACTIVE_EFFECT.set(self.previous);
    }
}

pub fn effect_context(effect_id: EffectId) -> EffectGuard {
    let previous = ACTIVE_EFFECT.get();
    ACTIVE_EFFECT.set(Some(effect_id));
    EffectGuard { previous }
}

pub fn current_effect() -> Option<EffectId> {
    ACTIVE_EFFECT.get()
}
