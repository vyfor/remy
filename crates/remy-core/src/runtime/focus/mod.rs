mod frame;
mod id;
mod nav;
mod state;
mod target;
mod trap;

pub use frame::{begin_focus_frame, finish_focus_frame};
pub use id::FocusId;
pub use nav::{focus_next, focus_prev};
pub use target::{
    add_focus_event, add_static_group_member, clear_focus, clear_focus_owner, current_focus_id,
    focus_id, focus_owner, get_focused_owner, is_focus_id, pop_group, present_focus, push_group,
};
pub use trap::{pop_trap, push_trap, trap_active};

pub(crate) use state::FocusState;
pub(crate) use target::remove_owner_focus;
pub(crate) use trap::{active_trap_has, active_trap_id, current_frame_trap_id};
