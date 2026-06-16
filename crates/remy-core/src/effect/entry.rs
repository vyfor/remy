use std::collections::HashSet;
use std::sync::Mutex;

use crate::state::SlotId;

use super::EffectId;

pub struct Effect {
    pub id: EffectId,
    pub callback: Box<dyn Fn() + Send + Sync>,
    pub tracked_slots: Mutex<HashSet<SlotId>>,
}
