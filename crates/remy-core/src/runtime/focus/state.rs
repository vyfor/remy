use std::collections::HashMap;

use crate::tracking::OwnerId;

use super::FocusId;

#[derive(Default)]
pub(crate) struct FocusState {
    pub(super) entries: Vec<FocusEntry>,
    pub(super) desired: Option<FocusId>,
    pub(super) current: Option<FocusId>,
    pub(super) groups: HashMap<FocusId, FocusGroup>,
    pub(super) active_group: Option<FocusId>,
    pub(super) capture_stack: Vec<&'static str>,
    pub(super) active_capture: Option<&'static str>,
    pub(super) captures: HashMap<&'static str, FocusScope>,
}

#[derive(Default)]
pub(super) struct FocusGroup {
    pub(super) entries: Vec<FocusEntry>,
    pub(super) desired: Option<FocusId>,
    pub(super) current: Option<FocusId>,
    pub(super) wrap: bool,
    pub(super) owner_id: OwnerId,
}

#[derive(Default)]
pub(super) struct FocusScope {
    pub(super) entries: Vec<FocusEntry>,
    pub(super) desired: Option<FocusId>,
    pub(super) current: Option<FocusId>,
}

#[derive(Clone, Copy)]
pub(super) struct FocusEntry {
    pub(super) id: FocusId,
    pub(super) owner_id: OwnerId,
}
