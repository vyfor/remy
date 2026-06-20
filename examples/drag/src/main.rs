use remy::core::keyboard::Key;
use remy::ratatui::buffer::Buffer;
use remy::ratatui::layout::Rect;
use remy::ratatui::prelude::Widget;
use remy::ratatui::style::{Color, Style};
use remy::ratatui::text::Line;
use remy::ratatui::widgets::{Block, Borders, Paragraph};
use remy::{Drag, Framework, MouseButton, Rcx, State, View, component, intent, quit, state, store};

const PANEL_W: u16 = 30;
const PANEL_H: u16 = 6;

#[store]
pub fn panel() {
    let x: State<u16> = state(0);
    let y: State<u16> = state(0);
    let drag_count: State<u32> = state(0);
    let press_count: State<u32> = state(0);
    let is_dragging: State<bool> = state(false);
}

#[intent]
fn move_up() {
    panel::y.update(|y| *y = y.saturating_sub(1));
}

#[intent]
fn move_down() {
    panel::y.update(|y| *y = y.saturating_add(1));
}

#[intent]
fn move_left() {
    panel::x.update(|x| *x = x.saturating_sub(1));
}

#[intent]
fn move_right() {
    panel::x.update(|x| *x = x.saturating_add(1));
}

#[intent]
fn press_panel() {
    panel::press_count.update(|c| *c += 1);
    panel::is_dragging.update(|d| *d = true);
}

#[intent]
fn release_panel() {
    panel::is_dragging.update(|d| *d = false);
}

#[intent]
fn drag_panel(dx: i16, dy: i16) {
    panel::drag_count.update(|c| *c += 1);
    panel::x.update(move |x| *x = x.saturating_add_signed(dx));
    panel::y.update(move |y| *y = y.saturating_add_signed(dy));
}

#[component]
fn FloatingPanel() {
    move |rcx: Rcx, buf: &mut Buffer, area: Rect| {
        let max_x = area.width.saturating_sub(PANEL_W);
        let max_y = area.height.saturating_sub(PANEL_H);
        let px = (*panel::x).min(max_x);
        let py = (*panel::y).min(max_y);

        let panel_area = Rect::new(px, py, PANEL_W, PANEL_H);

        rcx.mouse_region("panel", panel_area)
            .on_press(MouseButton::Left, press_panel)
            .on_release(MouseButton::Left, release_panel)
            .on_drag(MouseButton::Left, |drag: Drag| {
                drag_panel(drag.delta_x, drag.delta_y)
            });

        let color = if *panel::is_dragging {
            Color::Yellow
        } else {
            Color::Cyan
        };

        Paragraph::new(vec![
            Line::raw("drag me with the mouse!"),
            Line::raw(""),
            Line::raw(format!("pos: ({}, {})", *panel::x, *panel::y)),
            Line::raw(format!(
                "press: {}  drag: {}",
                *panel::press_count,
                *panel::drag_count
            )),
        ])
        .centered()
        .style(Style::default().fg(color))
        .block(Block::new().borders(Borders::ALL))
        .render(panel_area, buf);
    }
}

#[component]
fn App(cx: remy::Cx) {
    cx.keys()
        .on_press(Key::up(), move_up)
        .on_press(Key::down(), move_down)
        .on_press(Key::left(), move_left)
        .on_press(Key::right(), move_right)
        .on_press('q', quit);

    move |rcx: Rcx, buf: &mut Buffer, area: Rect| {
        FloatingPanel().render(rcx, buf, area);
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    Framework::new().enable_mouse().run(App).await
}
