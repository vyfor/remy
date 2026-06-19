use remy::ratatui::buffer::Buffer;
use remy::ratatui::layout::Rect;
use remy::ratatui::prelude::Widget;
use remy::ratatui::widgets::{Block, Borders, Paragraph};
use remy::{Framework, Rcx, State, View, component, intent, quit, state, store};

#[store]
pub fn counter() {
    let count: State<i32> = state(0);
}

#[intent]
fn increment() {
    counter::count.update(|count| *count += 1);
}

#[intent]
fn decrement() {
    counter::count.update(|count| *count -= 1);
}

#[component]
fn App(cx: remy::Cx) -> impl View {
    cx.on_press('+', increment);
    cx.on_press('-', decrement);
    cx.on_press('q', quit);

    move |_rcx: Rcx, buf: &mut Buffer, area: Rect| {
        let widget = Paragraph::new(format!("count: {}", *counter::count))
            .block(Block::new().title("counter").borders(Borders::ALL));

        widget.render(area, buf);
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    Framework::new().run(App).await
}
