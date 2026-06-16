use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::tracking::OwnerId;

use super::{Runtime, cancel_owner, remove_owner_focus};

pub(crate) fn next_id() -> u32 {
    let mut names = std::collections::HashSet::new();
    for &name in crate::OWNER_REGISTRY {
        names.insert(name);
    }
    if names.is_empty() {
        1
    } else {
        names.len() as u32 + 2
    }
}

pub fn register_owner(name: &'static str) -> OwnerId {
    static OWNER_ID_MAP: OnceLock<HashMap<&'static str, OwnerId>> = OnceLock::new();
    let map = OWNER_ID_MAP.get_or_init(|| {
        let mut seen = HashSet::new();
        crate::OWNER_REGISTRY
            .iter()
            .copied()
            .filter(|name| seen.insert(*name))
            .zip(2_u32..)
            .collect()
    });
    if let Some(&id) = map.get(name) {
        id
    } else {
        spawn_owner(name)
    }
}

pub fn spawn_owner(_name: &'static str) -> OwnerId {
    Runtime::get()
        .next_owner_id
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub fn set_active_owner(owner_id: Option<OwnerId>) -> Option<OwnerId> {
    crate::tracking::ACTIVE_OWNER.with(|c| {
        let prev = c.get();
        c.set(owner_id);
        prev
    })
}

pub fn dispose_owner(owner_id: OwnerId) {
    cancel_owner(owner_id);
    remove_owner_focus(owner_id);
}
