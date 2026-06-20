use std::collections::{HashMap, HashSet};

use crate::tracking::OwnerId;

use super::FocusId;
use crate::focus_builder::EventCallback;

#[derive(Default)]
pub(crate) struct FocusState {
    pub(super) focus_order: Vec<FocusEntry>,
    pub(super) presented: HashSet<OwnerId>,
    pub(super) group_stack: Vec<FocusId>,
    pub(super) group_entries: HashMap<FocusId, Vec<FocusEntry>>,
    pub(super) trap_stack: Vec<&'static str>,
    pub(super) trap_entries: HashMap<&'static str, Vec<FocusEntry>>,
    pub(super) static_events: HashMap<FocusId, StaticFocusEvents>,
    pub(super) static_groups: HashMap<FocusId, StaticGroup>,
    pub(super) desired: Option<FocusId>,
    pub(super) current: Option<FocusId>,
    pub(super) previous: Option<FocusId>,
    pub(super) active_trap: Option<&'static str>,
    pub(super) active_group: Option<FocusId>,
}

pub(crate) struct StaticFocusEvents {
    pub owner_id: OwnerId,
    pub on_focus: Option<EventCallback>,
    pub on_blur: Option<EventCallback>,
}

pub(crate) struct StaticGroup {
    pub owner_id: OwnerId,
    pub members: Vec<FocusId>,
    pub wrap: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct FocusEntry {
    pub(crate) id: FocusId,
    pub(crate) owner_id: OwnerId,
}
