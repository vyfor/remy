use std::any::Any;
use std::sync::{Arc, Mutex};

use crate::state::SlotId;

type Update = Box<dyn FnOnce(&crate::state::Slots, SlotId) + Send>;

pub struct Queue {
    pending: Mutex<Vec<Commit>>,
}

pub struct Commit {
    pub slot_id: SlotId,
    pub op: Op,
}

pub enum Op {
    Set(Arc<dyn Any + Send + Sync>),
    Update(Update),
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

impl Queue {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
        }
    }

    pub fn push<T: Send + Sync + 'static>(&self, slot_id: SlotId, value: T) {
        self.pending.lock().unwrap().push(Commit {
            slot_id,
            op: Op::Set(Arc::new(value)),
        });
    }

    pub fn push_update<T, F>(&self, slot_id: SlotId, f: F)
    where
        T: Send + Sync + Clone + 'static,
        F: FnOnce(&mut T) + Send + 'static,
    {
        self.pending.lock().unwrap().push(Commit {
            slot_id,
            op: Op::Update(Box::new(move |state, slot_id| {
                state.update_pending::<T, F>(slot_id, f);
            })),
        });
    }

    pub fn extend(&self, commits: Vec<Commit>) {
        if commits.is_empty() {
            return;
        }
        self.pending.lock().unwrap().extend(commits);
    }

    pub fn drain(&self) -> Vec<Commit> {
        let mut pending = self.pending.lock().unwrap();
        std::mem::take(&mut *pending)
    }
}
