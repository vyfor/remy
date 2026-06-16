# 🐀 remy

remy is an opinionated, reactive framework for **ratatui** -- a rust crate for cooking up terminal user interfaces. remy is a work in progress.

## why the name

the name `ratatui` is a play on the movie **ratatouille**.

in the movie, **remy** is the little rat who hides under chef linguini's hat, pulling his hair to manage the kitchen behind the scenes.

`remy` works the same. it handles the state, you handle the interface.

## features

- async support
- reactive state (effects, memos, resources, queries)
- input handling (keys, chords, mouse)
- focus
- overlays

## examples

```rs
use remy::ratatui::layout::Rect;
use remy::ratatui::widgets::{Block, Borders, Paragraph};
use remy::{Framework, State, View, component, intent, quit, state, store};

#[store]
pub fn counter() {
    let count: State<i32> = state(0);
}

#[intent]
fn increment() {
    counter::count.update(|c| *c += 1);
}

#[intent]
fn decrement() {
    counter::count.update(|c| *c -= 1);
}

#[component]
fn App() -> impl View {
    move |frame: &mut remy::ratatui::Frame, area: Rect| {
        let widget = Paragraph::new(format!("count: {}", *counter::count))
            .block(Block::new().title("counter").borders(Borders::ALL));

        frame.render_widget(widget, area);
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    Framework::new()
        .keys(|keys| {
            keys.bind('+', increment);
            keys.bind('-', decrement);
            keys.bind('q', quit);
        })
        .run(App)
        .await
}
```
