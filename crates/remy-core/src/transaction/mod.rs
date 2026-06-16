use std::any::Any;
use std::sync::Arc;

use crate::bus::{Commit, Op};
use crate::proxy::Proxy;
use crate::state::SlotId;

#[derive(Default)]
pub struct Transaction {
    commits: Vec<Commit>,
}

impl Transaction {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set<T>(&mut self, proxy: impl Into<Proxy<T>>, value: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.push_set(proxy.into().id(), value);
        self
    }

    pub fn update<T, F>(&mut self, proxy: impl Into<Proxy<T>>, f: F) -> &mut Self
    where
        T: Send + Sync + Clone + 'static,
        F: FnOnce(&mut T) + Send + 'static,
    {
        self.push_update::<T, F>(proxy.into().id(), f);
        self
    }

    pub fn commit(self) {
        crate::runtime::commit_transaction(self.commits);
    }

    pub fn is_empty(&self) -> bool {
        self.commits.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commits.len()
    }

    fn push_set<T>(&mut self, slot_id: SlotId, value: T)
    where
        T: Send + Sync + 'static,
    {
        self.commits.push(Commit {
            slot_id,
            op: Op::Set(Arc::new(value) as Arc<dyn Any + Send + Sync>),
        });
    }

    fn push_update<T, F>(&mut self, slot_id: SlotId, f: F)
    where
        T: Send + Sync + Clone + 'static,
        F: FnOnce(&mut T) + Send + 'static,
    {
        self.commits.push(Commit {
            slot_id,
            op: Op::Update(Box::new(move |state, slot_id| {
                state.update_pending::<T, F>(slot_id, f);
            })),
        });
    }
}

pub fn transaction() -> Transaction {
    Transaction::new()
}
