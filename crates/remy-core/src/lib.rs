pub mod bus;
pub mod cached;
pub mod cx;
pub mod effect;
pub mod focus;
pub mod focus_builder;
pub mod framework;
pub mod id;
pub mod instance;
pub mod key;
pub mod keyboard;
pub mod load;
pub mod memo;
pub mod mouse;
pub mod owner;
pub mod proxy;
pub mod query;
pub mod resource;
pub mod runtime;
pub mod scope;
pub mod state;
pub mod tracking;
pub mod transaction;
pub mod view;

pub use cached::CachedView;
pub use cx::{Cx, Rcx};
pub use focus_builder::{FocusBuilder, FocusGroupBuilder, RenderFocus};
pub use framework::{Framework, Text};
pub use keyboard::{Bind, BindKind, Chord, ChordPolicy, Flow, IntoBind, Key, Keys, Mods, quit};
pub use load::Load;
pub use memo::{Memo, memo};
pub use mouse::{Pos, Region, RegionBuilder, Scroll};
pub use owner::Owner;
pub use proxy::{Init, Proxy, State, install, state};
pub use query::{Query, QueryOpts, query};
pub use resource::{Refresh, Resource, ResourceOpts, Retry, resource};
pub use runtime::{FocusId, LayerHandle, LayerId};
pub use scope::{Scope, StoreCx};
pub use state::{SlotId, const_slot_id};
pub use transaction::{Transaction, transaction};
pub use view::View;

#[linkme::distributed_slice]
pub static STORE_REGISTRY: [fn(Scope)];

#[linkme::distributed_slice]
pub static INTENT_REGISTRY: [(u32, bus::IntentFn)];

#[linkme::distributed_slice]
pub static OWNER_REGISTRY: [&'static str];

#[linkme::distributed_slice]
pub static SLOT_REGISTRY: [(&'static str, &'static str, state::SlotId)];

pub fn check_slot_collisions() {
    use std::collections::HashMap;
    let mut seen: HashMap<state::SlotId, (&str, &str)> = HashMap::new();
    for &(module_path, var_name, slot_id) in SLOT_REGISTRY {
        match seen.entry(slot_id) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert((module_path, var_name));
            }
            std::collections::hash_map::Entry::Occupied(e) => {
                let (prev_module, prev_var) = *e.get();
                if prev_module != module_path || prev_var != var_name {
                    // todo: revisit later
                    panic!(
                        "id collision between `{prev_module}::{prev_var}` and `{module_path}::{var_name}`"
                    );
                }
            }
        }
    }
}

#[macro_export]
macro_rules! batch {
    ($($body:tt)*) => {{
        $crate::runtime::batch_enter();
        let __batch_result = { $($body)* };
        $crate::runtime::batch_exit();
        __batch_result
    }};
}
