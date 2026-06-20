mod frame;
mod id;
mod nav;
mod state;
mod target;
mod trap;

pub use id::FocusId;
pub use frame::{begin_focus_frame, finish_focus_frame};
pub use target::{
    clear_focus, clear_focus_owner, current_focus_id, focus_id, focus_owner,
    get_focused_owner, is_focus_id, present_focus, add_focus_event,
    add_static_group_member, push_group, pop_group,
};
pub use trap::{trap_active, push_trap, pop_trap};
pub use nav::{focus_next, focus_prev};

pub(crate) use trap::{active_trap_id, active_trap_has, current_frame_trap_id};
pub(crate) use state::FocusState;
pub(crate) use target::remove_owner_focus;
