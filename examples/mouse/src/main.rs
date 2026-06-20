use remy::ratatui::buffer::Buffer;
use remy::ratatui::layout::{Constraint, Rect};
use remy::ratatui::prelude::Widget;
use remy::ratatui::style::{Color, Style};
use remy::ratatui::text::Line;
use remy::ratatui::widgets::{Block, Borders, Paragraph};
use remy::{Framework, MouseButton, Rcx, State, View, component, intent, quit, state, store};

#[store]
pub fn clicks() {
    let left: State<u32> = state(0);
    let right: State<u32> = state(0);
}

#[intent]
fn left_click() {
    clicks::left.update(|c| *c += 1);
}

#[intent]
fn right_click() {
    clicks::right.update(|c| *c += 1);
}

#[component]
fn ClickBox(cx: remy::Cx) {
    cx.keys().on_press('q', quit);

    move |rcx: Rcx, buf: &mut Buffer, area: Rect| {
        let left = *clicks::left;
        let right = *clicks::right;

        let is_hovered = rcx
            .mouse_region("box", area)
            .on_click(MouseButton::Left, left_click)
            .on_click(MouseButton::Right, right_click)
            .hovered();

        let color = if is_hovered {
            Color::Yellow
        } else {
            Color::Cyan
        };

        Paragraph::new(vec![
            Line::raw(format!(" L: {} | R: {}", left, right)),
            Line::raw(""),
            Line::raw("click me"),
        ])
        .centered()
        .style(Style::default().fg(color))
        .block(Block::new().borders(Borders::ALL))
        .render(area, buf);
    }
}

#[component]
fn App() {
    move |rcx: Rcx, buf: &mut Buffer, area: Rect| {
        let rect = area.centered(Constraint::Length(24), Constraint::Length(5));
        ClickBox().render(rcx, buf, rect);
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    Framework::new().enable_mouse().run(App).await
}
