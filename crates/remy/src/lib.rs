pub use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
pub use remy_core as core;
pub use remy_core::batch;
pub use remy_core::keyboard::quit;
pub use remy_core::keyboard::{IntoBind, IntoFlow};
pub use remy_core::runtime::Runtime;
pub use remy_core::{
    Bind, BindKind, Chord, ChordPolicy, Cx, Flow, Framework, Init, Key, Keys, LayerHandle, LayerId,
    Load, Memo, Mods, Owner, Pos, Proxy, Query, QueryOpts, Refresh, Region, Resource, ResourceOpts,
    Retry, Scope, Scroll, SlotId, State, StoreCx, Text, Transaction, View, const_slot_id, memo,
    query, resource, state, transaction,
};
pub use remy_core::{INTENT_REGISTRY, STORE_REGISTRY};
pub use remy_core::{bus, effect, runtime, tracking};

pub mod focus {
    pub use remy_core::focus::{
        Focus, FocusGroup, FocusId, FocusTarget, active_group, capture, clear, current,
        enter_group, is, leave_group, next, next_group, prev, prev_group, set,
    };
}

pub mod keys {
    pub use remy_core::key::register;
    pub use remy_core::runtime::{
        chord_completions as completions, pending_chord as pending,
        pending_chord_label as pending_label,
    };
}

pub use linkme;
pub use ratatui;
pub use remy_macros::{component, intent, store};
