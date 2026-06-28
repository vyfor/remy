use crate::app::App;

mod executor;
mod queue;

pub use executor::Executor;
pub use queue::{Commit, Op, Queue};

pub type IntentId = u32;

pub type IntentFn = fn(App, Box<dyn std::any::Any + Send>);
