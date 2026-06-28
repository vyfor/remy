pub use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
pub use ratatui::layout::Position;
pub use remy_core as core;
pub use remy_core::Drag;
pub use remy_core::batch;
pub use remy_core::frame_interval;
pub use remy_core::keyboard::quit;
pub use remy_core::keyboard::{IntoBind, IntoFlow};
pub use remy_core::runtime::Runtime;
pub use remy_core::set_frame_interval;
pub use remy_core::set_frame_rate;
pub use remy_core::tracking::set_cursor_position;
pub use remy_core::{
    App, Cx, Memo, Query, Rcx, Resource, State,
    effect::Effect,
    focus_builder::{FocusBuilder, FocusGroupBuilder, RenderFocus},
    framework::Framework,
    id::Id,
    instance::{Instance, hash_props},
    memo, query, resource, state,
    transaction::transaction,
    view::View,
};
pub use remy_core::{INTENT_REGISTRY, STORE_REGISTRY};
pub use remy_core::{bus, effect, runtime, tracking};

// ratatui buffer does not provide a way to update cursor
pub fn set_cursor(position: impl Into<Position>) {
    set_cursor_position(Some(position.into()));
}

pub fn hide_cursor() {
    set_cursor_position(None);
}

pub mod focus {
    pub use remy_core::focus::{clear, current, is, next, prev, set};
    pub use remy_core::runtime::FocusId;
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
