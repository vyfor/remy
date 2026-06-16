use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::keyboard::Flow;
use crate::runtime::FocusId;

use super::{Pos, Region, Scroll};

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

    pub fn finish_frame(&mut self, active_capture: Option<&'static str>) -> bool {
        self.recompute_hover(active_capture)
    }

    pub fn dispatch_event(
        &mut self,
        event: &MouseEvent,
        active_capture: Option<&'static str>,
    ) -> (Flow, bool, Option<FocusId>) {
        let pos = Pos::from_event(event);
        self.latest_position = Some(pos);

        match event.kind {
            MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                let hover_changed = self.recompute_hover(active_capture);
                (Flow::Handled, hover_changed, None)
            }
            MouseEventKind::Down(button) => {
                let region = self.hit_region(pos, active_capture).cloned();
                if let Some(region) = region {
                    self.pressed = Some(PressedRegion {
                        id: region.id,
                        button,
                    });
                    if let Some(action) = region.press_action(button) {
                        return (action(), false, None);
                    }
                    if region.has_click_interest(button) {
                        return (Flow::Handled, false, None);
                    }
                }
                (Flow::Ignored, false, None)
            }
            MouseEventKind::Up(button) => {
                let region = self.hit_region(pos, active_capture).cloned();
                let pressed = self.pressed.take();
                let Some(region) = region else {
                    return (Flow::Ignored, false, None);
                };

                let mut result = region
                    .release_action(button)
                    .map(|action| action())
                    .unwrap_or(Flow::Ignored);

                let is_click = pressed
                    == Some(PressedRegion {
                        id: region.id,
                        button,
                    });
                if is_click {
                    let focus = region.focus_on_click.then_some(region.id);
                    if let Some(action) = region.click_action(button) {
                        result = combine_results(result, action());
                    } else if region.focus_on_click {
                        result = combine_results(result, Flow::Handled);
                    }
                    return (result, false, focus);
                }

                (result, false, None)
            }
            MouseEventKind::ScrollUp => self.dispatch_scroll(
                pos,
                active_capture,
                Scroll {
                    delta_x: 0,
                    delta_y: 1,
                },
            ),
            MouseEventKind::ScrollDown => self.dispatch_scroll(
                pos,
                active_capture,
                Scroll {
                    delta_x: 0,
                    delta_y: -1,
                },
            ),
            MouseEventKind::ScrollLeft => self.dispatch_scroll(
                pos,
                active_capture,
                Scroll {
                    delta_x: -1,
                    delta_y: 0,
                },
            ),
            MouseEventKind::ScrollRight => self.dispatch_scroll(
                pos,
                active_capture,
                Scroll {
                    delta_x: 1,
                    delta_y: 0,
                },
            ),
        }
    }

    fn dispatch_scroll(
        &self,
        pos: Pos,
        active_capture: Option<&'static str>,
        scroll: Scroll,
    ) -> (Flow, bool, Option<FocusId>) {
        let result = self
            .hit_region(pos, active_capture)
            .and_then(|region| region.scroll.as_ref())
            .map(|handler| handler(scroll))
            .unwrap_or(Flow::Ignored);
        (result, false, None)
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
        if self.hovered == hovered {
            return false;
        }
        self.hovered = hovered;
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

fn combine_results(left: Flow, right: Flow) -> Flow {
    match (left, right) {
        (Flow::Quit, _) | (_, Flow::Quit) => Flow::Quit,
        (Flow::Handled, _) | (_, Flow::Handled) => Flow::Handled,
        (Flow::Ignored, Flow::Ignored) => Flow::Ignored,
    }
}
