use crate::scope::Scope;

mod executor;
mod queue;

pub use executor::Executor;
pub use queue::{Commit, Op, Queue};

pub type IntentId = u32;

pub type IntentFn = fn(Scope, Box<dyn std::any::Any + Send>);
