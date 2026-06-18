use std::sync::Arc;

use crossterm::event::MouseButton;
use ratatui::layout::Rect;

use crate::keyboard::IntoFlow;
use crate::runtime::FocusId;
use crate::tracking::OwnerId;

use super::{MouseAction, Pos, Scroll, ScrollAction};

#[derive(Clone)]
pub struct Region {
    pub(crate) id: FocusId,
    pub(crate) area: Rect,
    pub owner_id: Option<OwnerId>,
    pub(crate) capture_id: Option<&'static str>,
    pub focus_on_click: bool,
    pub wants_hover: bool,
    pub(crate) clicks: Vec<(MouseButton, MouseAction)>,
    pub(crate) presses: Vec<(MouseButton, MouseAction)>,
    pub(crate) releases: Vec<(MouseButton, MouseAction)>,
    pub(crate) scroll: Option<ScrollAction>,
}

impl Region {
    pub fn new(id: FocusId, area: Rect) -> Self {
        Self {
            id,
            area,
            owner_id: None,
            capture_id: None,
            focus_on_click: false,
            wants_hover: false,
            clicks: Vec::new(),
            presses: Vec::new(),
            releases: Vec::new(),
            scroll: None,
        }
    }

    pub fn id(&self) -> FocusId {
        self.id
    }

    pub fn focus_on_click(&mut self) {
        self.focus_on_click = true;
    }

    pub fn wants_hover(&mut self) {
        self.wants_hover = true;
    }

    pub fn on_click<F, R>(&mut self, button: MouseButton, handler: F)
    where
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.clicks
            .push((button, Arc::new(move || handler().into_key_result())));
    }

    pub fn on_press<F, R>(&mut self, button: MouseButton, handler: F)
    where
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.presses
            .push((button, Arc::new(move || handler().into_key_result())));
    }

    pub fn on_release<F, R>(&mut self, button: MouseButton, handler: F)
    where
        F: Fn() -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.releases
            .push((button, Arc::new(move || handler().into_key_result())));
    }

    pub fn on_scroll<F, R>(&mut self, handler: F)
    where
        F: Fn(Scroll) -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.scroll = Some(Arc::new(move |scroll| handler(scroll).into_key_result()));
    }

    pub(crate) fn attach_runtime(
        &mut self,
        owner_id: Option<OwnerId>,
        capture_id: Option<&'static str>,
    ) {
        self.owner_id = owner_id;
        self.capture_id = capture_id;
    }

    pub(crate) fn contains(&self, pos: Pos) -> bool {
        pos.column >= self.area.x
            && pos.column < self.area.x.saturating_add(self.area.width)
            && pos.row >= self.area.y
            && pos.row < self.area.y.saturating_add(self.area.height)
    }

    fn button_action(
        actions: &[(MouseButton, MouseAction)],
        button: MouseButton,
    ) -> Option<MouseAction> {
        actions
            .iter()
            .find(|entry| entry.0 == button)
            .map(|entry| Arc::clone(&entry.1))
    }

    pub(crate) fn press_action(&self, button: MouseButton) -> Option<MouseAction> {
        Self::button_action(&self.presses, button)
    }

    pub(crate) fn release_action(&self, button: MouseButton) -> Option<MouseAction> {
        Self::button_action(&self.releases, button)
    }

    pub(crate) fn click_action(&self, button: MouseButton) -> Option<MouseAction> {
        Self::button_action(&self.clicks, button)
    }

    pub(crate) fn has_click_interest(&self, button: MouseButton) -> bool {
        self.focus_on_click || Self::button_action(&self.clicks, button).is_some()
    }
}