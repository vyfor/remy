pub mod bus;
pub mod app;
pub mod cached;
pub mod cx;
pub mod effect;
pub mod focus;
pub mod focus_builder;
pub mod framework;
pub mod handle;
pub mod id;
pub mod instance;
pub mod key;
pub mod keyboard;
pub mod load;
pub mod memo;
pub mod mouse;
pub mod owner;
pub mod query;
pub mod resource;
pub mod runtime;
pub mod state;
pub mod tracking;
pub mod transaction;
pub mod view;

pub use app::App;
pub use cached::CachedView;
pub use cx::{Cx, Rcx};
pub use focus_builder::{FocusBuilder, FocusGroupBuilder, RenderFocus};
pub use framework::Framework;
pub use keyboard::{Bind, BindKind, Chord, ChordPolicy, Flow, IntoBind, Key, Keys, Mods, quit};
pub use load::Load;
pub use memo::{Memo, memo};
pub use mouse::{Drag, Pos, Region, RegionBuilder, Scroll};
pub use owner::Owner;
pub use handle::{Init, State, install, state};
pub use query::{Query, QueryOpts, query};
pub use resource::{Refresh, Resource, ResourceOpts, Retry, resource};
pub use runtime::{
    FocusId, LayerHandle, LayerId, frame_interval, set_frame_interval, set_frame_rate,
};
pub use state::SlotId;
pub use transaction::{Transaction, transaction};
pub use view::View;

#[linkme::distributed_slice]
pub static STORE_REGISTRY: [fn(App)];

#[linkme::distributed_slice]
pub static INTENT_REGISTRY: [(&'static std::sync::OnceLock<u32>, bus::IntentFn)];

#[linkme::distributed_slice]
pub static OWNER_REGISTRY: [&'static str];

#[macro_export]
macro_rules! batch {
    ($($body:tt)*) => {{
        $crate::runtime::batch_enter();
        let __batch_result = { $($body)* };
        $crate::runtime::batch_exit();
        __batch_result
    }};
}
