use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tokio::task::JoinHandle;

use super::IntentId;

pub struct Executor {
    handles: Mutex<HashMap<IntentId, JoinHandle<()>>>,
    last_run: Mutex<HashMap<IntentId, Instant>>,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    pub fn new() -> Self {
        Self {
            handles: Mutex::new(HashMap::new()),
            last_run: Mutex::new(HashMap::new()),
        }
    }

    pub fn dispatch<F>(&self, id: IntentId, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let join = tokio::spawn(fut);
        let mut handles = self.handles.lock().unwrap();
        if let Some(old) = handles.insert(id, join) {
            old.abort();
        }
    }

    pub fn dispatch_debounced<F>(&self, id: IntentId, delay: Duration, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if let Some(old) = self.handles.lock().unwrap().remove(&id) {
            old.abort();
        }
        let join = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            fut.await;
        });
        self.handles.lock().unwrap().insert(id, join);
    }

    pub fn dispatch_throttled<F>(&self, id: IntentId, interval: Duration, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let now = Instant::now();
        {
            let mut last_run = self.last_run.lock().unwrap();
            if let Some(&last) = last_run.get(&id)
                && now.duration_since(last) < interval
            {
                return;
            }
            last_run.insert(id, now);
        }
        self.dispatch(id, fut);
    }

    pub fn cancel(&self, id: IntentId) {
        if let Some(old) = self.handles.lock().unwrap().remove(&id) {
            old.abort();
        }
    }

    pub fn sweep(&self) {
        self.handles.lock().unwrap().retain(|_, h| !h.is_finished());
    }
}
