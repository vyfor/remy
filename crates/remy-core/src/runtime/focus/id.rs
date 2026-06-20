use std::hash::{Hash, Hasher};

use crate::tracking::OwnerId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(u64);

impl FocusId {
    pub fn new<T: Hash>(value: T) -> Self {
        let mut hasher = rapidhash::fast::RapidHasher::default();
        value.hash(&mut hasher);
        Self(hasher.finish())
    }

    pub(crate) fn component(owner_id: OwnerId) -> Self {
        Self::new(("owner", owner_id))
    }
}
