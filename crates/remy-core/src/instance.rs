use std::hash::{Hash, Hasher};
use std::sync::Arc;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::cached::CachedView;
use crate::cx::Rcx;
use crate::tracking::OwnerId;
use crate::view::View;

pub struct Instance {
    pub(crate) owner_id: OwnerId,
    view: Arc<dyn View + Send + Sync>,
}

impl Instance {
    pub fn new<V>(owner_id: OwnerId, view: V) -> Self
    where
        V: View + Send + Sync + 'static,
    {
        Self {
            owner_id,
            view: Arc::new(CachedView::new(owner_id, view)),
        }
    }

    #[doc(hidden)]
    pub fn __new_raw(owner_id: OwnerId, view: Arc<dyn View + Send + Sync>) -> Self {
        Self { owner_id, view }
    }

    pub fn owner_id(&self) -> OwnerId {
        self.owner_id
    }
}

impl Clone for Instance {
    fn clone(&self) -> Self {
        Self {
            owner_id: self.owner_id,
            view: Arc::clone(&self.view),
        }
    }
}

impl View for Instance {
    fn render(&self, rcx: Rcx, buf: &mut Buffer, area: Rect) {
        self.view.render(rcx, buf, area);
    }
}

pub fn hash_props<T: Hash>(t: &T) -> u64 {
    let mut hasher = rapidhash::fast::RapidHasher::default();
    t.hash(&mut hasher);
    hasher.finish()
}
