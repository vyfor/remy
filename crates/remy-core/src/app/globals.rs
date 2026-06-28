use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Globals {
    map: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
}

impl Default for Globals {
    fn default() -> Self {
        Self::new()
    }
}

impl Globals {
    pub fn new() -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn provide<T: Send + Sync + 'static>(&self, value: T) {
        self.map
            .write()
            .unwrap()
            .insert(TypeId::of::<T>(), Arc::new(value));
    }

    pub fn get<T: Send + Sync + Clone + 'static>(&self) -> Option<T> {
        self.map
            .read()
            .unwrap()
            .get(&TypeId::of::<T>())
            .and_then(|a| a.downcast_ref::<T>().cloned())
    }

    pub fn get_arc<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.map
            .read()
            .unwrap()
            .get(&TypeId::of::<T>())
            .and_then(|a| Arc::clone(a).downcast::<T>().ok())
    }
}
