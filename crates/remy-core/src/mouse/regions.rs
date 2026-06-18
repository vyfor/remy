use std::collections::HashSet;
use std::sync::Arc;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::runtime::FocusId;
use crate::tracking::OwnerId;

use super::{MouseAction, Pos, Region, Scroll};

pub enum DispatchResult {
    None,
    Single(MouseAction),
    Multiple(Vec<MouseAction>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PressedRegion {
    id: FocusId,
    button: MouseButton,
}

#[derive(Default)]
pub struct Regions {
    regions: Vec<Region>,
    latest_position: Option<Pos>,
    hovered: Option<FocusId>,
    pressed: Option<PressedRegion>,
    last_hovered_owner: Option<OwnerId>,
}

impl Regions {
    pub fn begin_frame(&mut self) {
        self.regions.clear();
    }

    pub fn register_region(&mut self, region: Region) {
        self.regions.push(region);
    }

    pub fn is_hovered(&self, id: FocusId) -> bool {
        self.hovered == Some(id)
    }

    pub fn finish_frame(
        &mut self,
        active_capture: Option<&'static str>,
    ) -> (bool, HashSet<OwnerId>) {
        let prev_owner = self.last_hovered_owner;
        let hover_changed = self.recompute_hover(active_capture);
        let mut owners: HashSet<OwnerId> = HashSet::new();
        if prev_owner != self.last_hovered_owner {
            if let Some(o) = prev_owner {
                owners.insert(o);
            }
            if let Some(o) = self.last_hovered_owner {
                owners.insert(o);
            }
        }
        (hover_changed, owners)
    }

    pub fn hovered_owner_ids(&self) -> HashSet<OwnerId> {
        self.last_hovered_owner.into_iter().collect()
    }

    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    pub fn dispatch_event(
        &mut self,
        event: &MouseEvent,
        active_capture: Option<&'static str>,
    ) -> (DispatchResult, bool, Option<FocusId>) {
        let pos = Pos::from_event(event);
        self.latest_position = Some(pos);

        match event.kind {
            MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                let hover_changed = self.recompute_hover(active_capture);
                (DispatchResult::None, hover_changed, None)
            }
            MouseEventKind::Down(button) => {
                let region = self.hit_region(pos, active_capture).cloned();
                if let Some(region) = region {
                    self.pressed = Some(PressedRegion {
                        id: region.id,
                        button,
                    });
                    if let Some(action) = region.press_action(button) {
                        return (DispatchResult::Single(action), false, None);
                    }
                    if region.has_click_interest(button) {
                        return (DispatchResult::None, false, None);
                    }
                }
                (DispatchResult::None, false, None)
            }
            MouseEventKind::Up(button) => {
                let region = self.hit_region(pos, active_capture).cloned();
                let pressed = self.pressed.take();
                let Some(region) = region else {
                    return (DispatchResult::None, false, None);
                };

                let release_action = region.release_action(button);
                let is_click = pressed
                    == Some(PressedRegion {
                        id: region.id,
                        button,
                    });

                if is_click {
                    let focus = region.focus_on_click.then_some(region.id);
                    let click_action = region.click_action(button);
                    let actions: Vec<_> = release_action
                        .into_iter()
                        .chain(click_action)
                        .collect();
                    if actions.is_empty() && region.focus_on_click {
                        return (DispatchResult::None, false, focus);
                    }
                    return (DispatchResult::Multiple(actions), false, focus);
                }

                match release_action {
                    Some(action) => (DispatchResult::Single(action), false, None),
                    None => (DispatchResult::None, false, None),
                }
            }
            MouseEventKind::ScrollUp => self.dispatch_scroll(
                pos,
                active_capture,
                Scroll { delta_x: 0, delta_y: 1 },
            ),
            MouseEventKind::ScrollDown => self.dispatch_scroll(
                pos,
                active_capture,
                Scroll { delta_x: 0, delta_y: -1 },
            ),
            MouseEventKind::ScrollLeft => self.dispatch_scroll(
                pos,
                active_capture,
                Scroll { delta_x: -1, delta_y: 0 },
            ),
            MouseEventKind::ScrollRight => self.dispatch_scroll(
                pos,
                active_capture,
                Scroll { delta_x: 1, delta_y: 0 },
            ),
        }
    }

    fn dispatch_scroll(
        &self,
        pos: Pos,
        active_capture: Option<&'static str>,
        scroll: Scroll,
    ) -> (DispatchResult, bool, Option<FocusId>) {
        let handler = self
            .hit_region(pos, active_capture)
            .and_then(|region| region.scroll.clone());

        match handler {
            Some(h) => {
                let action: MouseAction = Arc::new(move || h(scroll));
                (DispatchResult::Single(action), false, None)
            }
            None => (DispatchResult::None, false, None),
        }
    }

    fn recompute_hover(&mut self, active_capture: Option<&'static str>) -> bool {
        let hovered = self.latest_position.and_then(|pos| {
            self.regions
                .iter()
                .rev()
                .find(|region| {
                    region.wants_hover
                        && Self::capture_allows(region.capture_id, active_capture)
                        && region.contains(pos)
                })
                .map(|region| region.id)
        });
        let owner = hovered.and_then(|id| {
            self.regions
                .iter()
                .find(|r| r.id == id)
                .and_then(|r| r.owner_id)
        });
        if self.hovered == hovered {
            return false;
        }
        self.hovered = hovered;
        self.last_hovered_owner = owner;
        true
    }

    fn hit_region(&self, pos: Pos, active_capture: Option<&'static str>) -> Option<&Region> {
        self.regions.iter().rev().find(|region| {
            Self::capture_allows(region.capture_id, active_capture) && region.contains(pos)
        })
    }

    fn capture_allows(
        region_capture: Option<&'static str>,
        active_capture: Option<&'static str>,
    ) -> bool {
        match active_capture {
            Some(active) => region_capture == Some(active),
            None => true,
        }
    }
}
