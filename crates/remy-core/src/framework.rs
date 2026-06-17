use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, Event,
    EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures_util::{FutureExt, StreamExt};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::keyboard::{self, Chord, ChordPolicy, Flow, IntoFlow, Key, Keys};
use crate::runtime;
use crate::scope::Globals;
use crate::view::View;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Text {
    Char(char),
    Paste(String),
}

pub struct Framework {
    globals: Globals,
    key_bindings: Keys,
    input_handlers: Inputs,
}

type ResizeHandler = Arc<dyn Fn(u16, u16) -> Flow + Send + Sync>;
type KeyHandler = Arc<dyn Fn(Key) -> Flow + Send + Sync>;
type PasteHandler = Arc<dyn Fn(String) -> Flow + Send + Sync>;
type TextInputHandler = Arc<dyn Fn(Text) -> Flow + Send + Sync>;
type MouseHandler = Arc<dyn Fn(MouseEvent) -> Flow + Send + Sync>;

#[derive(Clone, Default)]
struct Inputs {
    resize: Option<ResizeHandler>,
    key_repeat: Option<KeyHandler>,
    key_release: Option<KeyHandler>,
    paste: Option<PasteHandler>,
    text_input: Option<TextInputHandler>,
    mouse: Option<MouseHandler>,
    mouse_regions: bool,
}

impl Inputs {
    fn wants_mouse(&self) -> bool {
        self.mouse_regions || self.mouse.is_some()
    }

    fn wants_mouse_regions(&self) -> bool {
        self.mouse_regions
    }

    fn wants_paste(&self) -> bool {
        self.paste.is_some() || self.text_input.is_some()
    }

    fn resize(&self, columns: u16, rows: u16) -> Flow {
        self.resize
            .as_ref()
            .map(|handler| handler(columns, rows))
            .unwrap_or(Flow::Ignored)
    }

    fn key_repeat(&self, key: Key) -> Flow {
        self.key_repeat
            .as_ref()
            .map(|handler| handler(key))
            .unwrap_or(Flow::Ignored)
    }

    fn key_release(&self, key: Key) -> Flow {
        self.key_release
            .as_ref()
            .map(|handler| handler(key))
            .unwrap_or(Flow::Ignored)
    }

    fn paste(&self, text: String) -> Flow {
        self.paste
            .as_ref()
            .map(|handler| handler(text))
            .unwrap_or(Flow::Ignored)
    }

    fn text(&self, input: Text) -> Flow {
        self.text_input
            .as_ref()
            .map(|handler| handler(input))
            .unwrap_or(Flow::Ignored)
    }

    fn mouse(&self, event: MouseEvent) -> Flow {
        self.mouse
            .as_ref()
            .map(|handler| handler(event))
            .unwrap_or(Flow::Ignored)
    }
}

impl Default for Framework {
    fn default() -> Self {
        Self::new()
    }
}

impl Framework {
    pub fn new() -> Self {
        let globals = Globals::new();
        runtime::Runtime::init(globals);
        Self {
            globals: runtime::Runtime::get().globals.clone(),
            key_bindings: Keys::new(),
            input_handlers: Inputs::default(),
        }
    }

    pub fn keys(mut self, configure: impl FnOnce(&mut Keys)) -> Self {
        configure(&mut self.key_bindings);
        self
    }

    pub fn enable_mouse(mut self) -> Self {
        self.input_handlers.mouse_regions = true;
        self
    }

    pub fn on_resize<F, R>(mut self, handler: F) -> Self
    where
        F: Fn(u16, u16) -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.input_handlers.resize = Some(Arc::new(move |columns, rows| {
            handler(columns, rows).into_key_result()
        }));
        self
    }

    pub fn on_key_repeat<F, R>(mut self, handler: F) -> Self
    where
        F: Fn(Key) -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.input_handlers.key_repeat = Some(Arc::new(move |key| handler(key).into_key_result()));
        self
    }

    pub fn on_key_release<F, R>(mut self, handler: F) -> Self
    where
        F: Fn(Key) -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.input_handlers.key_release = Some(Arc::new(move |key| handler(key).into_key_result()));
        self
    }

    pub fn on_paste<F, R>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.input_handlers.paste = Some(Arc::new(move |text| handler(text).into_key_result()));
        self
    }

    pub fn on_text_input<F, R>(mut self, handler: F) -> Self
    where
        F: Fn(Text) -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.input_handlers.text_input =
            Some(Arc::new(move |input| handler(input).into_key_result()));
        self
    }

    pub fn on_mouse<F, R>(mut self, handler: F) -> Self
    where
        F: Fn(MouseEvent) -> R + Send + Sync + 'static,
        R: IntoFlow + 'static,
    {
        self.input_handlers.mouse = Some(Arc::new(move |event| handler(event).into_key_result()));
        self
    }

    pub fn provide_global<T: Send + Sync + 'static>(self, value: T) -> Self {
        self.globals.provide(value);
        self
    }

    pub async fn run<V: View + 'static>(self, root: impl FnOnce() -> V) -> io::Result<()> {
        tokio::runtime::Handle::try_current()
            .expect("remy::Framework::run needs a running tokio runtime");

        let root = root();
        let key_bindings = self.key_bindings;
        let input_handlers = self.input_handlers;

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        enter_terminal(&mut stdout, &input_handlers)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = run_loop(&mut terminal, &root, &key_bindings, &input_handlers).await;

        leave_terminal(&mut terminal, &input_handlers)?;
        terminal.show_cursor()?;
        res
    }
}

fn enter_terminal(stdout: &mut io::Stdout, input_handlers: &Inputs) -> io::Result<()> {
    execute!(stdout, EnterAlternateScreen)?;
    if input_handlers.wants_mouse() {
        execute!(stdout, EnableMouseCapture)?;
    }
    if input_handlers.wants_paste() {
        execute!(stdout, EnableBracketedPaste)?;
    }
    Ok(())
}

fn leave_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    input_handlers: &Inputs,
) -> io::Result<()> {
    if input_handlers.wants_paste() {
        execute!(terminal.backend_mut(), DisableBracketedPaste)?;
    }
    if input_handlers.wants_mouse() {
        execute!(terminal.backend_mut(), DisableMouseCapture)?;
    }
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

async fn run_loop<V: View>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    root: &V,
    key_bindings: &Keys,
    input_handlers: &Inputs,
) -> io::Result<()> {
    let mut terminal_events = EventStream::new();
    const EVENT_DRAIN_BUDGET: usize = 64;
    runtime::set_global_keys(key_bindings.clone());
    let mut needs_render = true;

    loop {
        let dirty_slots = runtime::apply_commits();
        runtime::begin_focus_frame();
        runtime::begin_keys();
        runtime::begin_mouse_frame();

        let needs_draw =
            needs_render || runtime::take_redraw() || runtime::should_render(&dirty_slots);

        if !needs_draw {
            runtime::finish_focus_frame();
            runtime::finish_mouse_frame();
            runtime::cancel_stale_chord();
            runtime::Runtime::get().executor.sweep();
        } else {
            crate::tracking::set_dirty_slots(dirty_slots.clone());
            crate::tracking::begin_render_tracking();
            runtime::begin_render();
            let draw = terminal.draw(|frame| {
                root.render(frame, frame.area());
                let overlays = runtime::drain_overlays();
                for entry in overlays {
                    (entry.render)(frame, entry.rect);
                }
            });
            runtime::end_render();
            crate::tracking::end_render_tracking();
            runtime::flush_render_reads();
            draw?;
            runtime::finish_focus_frame();
            runtime::finish_mouse_frame();
            runtime::cancel_stale_chord();
            runtime::Runtime::get().executor.sweep();
        }

        needs_render = false;

        if dispatch_timeout(key_bindings) == Flow::Quit {
            return Ok(());
        }

        let sleep_for = sleep_duration();
        let event = tokio::select! {
            biased;
            event = terminal_events.next() => event.transpose()?,
            _ = runtime::dirty_notify().notified() => None,
            _ = tokio::time::sleep(sleep_for) => None,
        };
        if dispatch_timeout(key_bindings) == Flow::Quit {
            return Ok(());
        }

        if let Some(event) = event {
            needs_render = true;
            if dispatch_event(event, key_bindings, input_handlers) == Flow::Quit {
                return Ok(());
            }
            for _ in 0..EVENT_DRAIN_BUDGET {
                let Some(event) = terminal_events.next().now_or_never() else {
                    break;
                };
                let Some(event) = event.transpose()? else {
                    break;
                };
                if dispatch_event(event, key_bindings, input_handlers) == Flow::Quit {
                    return Ok(());
                }
            }
        }
    }
}

fn dispatch_event(event: Event, key_bindings: &Keys, input_handlers: &Inputs) -> Flow {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            if let Some(ch) = char_from_ev(&key) {
                match input_handlers.text(Text::Char(ch)) {
                    Flow::Ignored => {}
                    result => return result,
                }
            }
            if let Some(key) = Key::from_event(key) {
                dispatch_key(key, key_bindings)
            } else {
                Flow::Ignored
            }
        }
        Event::Key(key) if key.kind == KeyEventKind::Repeat => {
            if let Some(key) = Key::from_event(key) {
                input_handlers.key_repeat(key)
            } else {
                Flow::Ignored
            }
        }
        Event::Key(key) if key.kind == KeyEventKind::Release => {
            if let Some(key) = Key::from_event(key) {
                input_handlers.key_release(key)
            } else {
                Flow::Ignored
            }
        }
        Event::Resize(columns, rows) => input_handlers.resize(columns, rows),
        Event::Paste(text) => match input_handlers.text(Text::Paste(text.clone())) {
            Flow::Ignored => input_handlers.paste(text),
            result => result,
        },
        Event::Mouse(event) => dispatch_mouse(event, input_handlers),
        _ => Flow::Ignored,
    }
}

fn char_from_ev(event: &KeyEvent) -> Option<char> {
    if event.modifiers.contains(KeyModifiers::CONTROL)
        || event.modifiers.contains(KeyModifiers::ALT)
    {
        return None;
    }
    match event.code {
        KeyCode::Char(ch) => Some(ch),
        _ => None,
    }
}

fn dispatch_mouse(event: MouseEvent, input_handlers: &Inputs) -> Flow {
    if input_handlers.wants_mouse_regions() {
        match runtime::dispatch_mouse_event(&event) {
            Flow::Ignored => input_handlers.mouse(event),
            result => result,
        }
    } else {
        input_handlers.mouse(event)
    }
}

fn sleep_duration() -> Duration {
    const FRAME_SLEEP: Duration = Duration::from_millis(16);
    runtime::chord_deadline()
        .map(|deadline| {
            deadline
                .saturating_duration_since(Instant::now())
                .min(FRAME_SLEEP)
        })
        .unwrap_or(FRAME_SLEEP)
}

fn dispatch_key(key: Key, global_bindings: &Keys) -> Flow {
    if let Some(pending) = runtime::pending_chord() {
        if runtime::chord_stale(pending.origin) {
            runtime::reset_chord();
        } else if let Some(expired) = runtime::take_expired_chord() {
            if fallback_first(expired.origin, expired.first_key, global_bindings) == Flow::Quit {
                return Flow::Quit;
            }
        } else {
            return dispatch_pending(key, global_bindings, pending);
        }
    }

    for (id, bindings) in runtime::focus_keys() {
        match dispatch_layer(key, &bindings, runtime::ChordOrigin::Focus(id)) {
            Some(Flow::Ignored) | None => {}
            Some(result) => return result,
        }
    }

    for (id, bindings) in runtime::view_keys() {
        match dispatch_layer(key, &bindings, runtime::ChordOrigin::View(id)) {
            Some(Flow::Ignored) | None => {}
            Some(result) => return result,
        }
    }

    if runtime::capture_active() {
        return Flow::Handled;
    }

    for (layer_id, layer_bindings) in runtime::layers() {
        let origin = runtime::ChordOrigin::Layer(layer_id);
        match dispatch_layer(key, &layer_bindings, origin) {
            Some(Flow::Ignored) | None => {}
            Some(result) => return result,
        }
    }

    dispatch_layer(key, global_bindings, runtime::ChordOrigin::Global).unwrap_or(Flow::Handled)
}

fn dispatch_pending(key: Key, global_bindings: &Keys, pending: runtime::PendingChord) -> Flow {
    if key == Key::esc() {
        let Some(first_key) = pending.keys.first() else {
            runtime::reset_chord();
            return Flow::Handled;
        };
        runtime::reset_chord();
        return fallback_first(pending.origin, first_key, global_bindings);
    }

    if pending.keys.len() >= keyboard::MAX_CHORD_LEN {
        runtime::reset_chord();
        return match pending.policy {
            ChordPolicy::Discard => Flow::Handled,
            ChordPolicy::Fallback => pending
                .keys
                .first()
                .map(|first_key| fallback_first(pending.origin, first_key, global_bindings))
                .unwrap_or(Flow::Handled),
        };
    }

    let next = pending.keys.clone().then(key);
    let Some(bindings) = bindings_for(pending.origin, global_bindings) else {
        runtime::reset_chord();
        return Flow::Handled;
    };

    if let Some(result) = bindings.dispatch_chord(&next) {
        runtime::reset_chord();
        return result;
    }

    if bindings.is_chord_prefix(&next) {
        let timeout = bindings.timeout_for_prefix(&next);
        let policy = bindings.policy_for_prefix(&next);
        runtime::update_chord(next, timeout, policy);
        return Flow::Handled;
    }

    runtime::reset_chord();
    match pending.policy {
        ChordPolicy::Discard => Flow::Handled,
        ChordPolicy::Fallback => pending
            .keys
            .first()
            .map(|first_key| fallback_first(pending.origin, first_key, global_bindings))
            .unwrap_or(Flow::Handled),
    }
}

fn dispatch_layer(key: Key, bindings: &Keys, origin: runtime::ChordOrigin) -> Option<Flow> {
    if bindings.has_chords() {
        let prefix = Chord::from(key);
        if bindings.is_chord_prefix(&prefix) {
            let timeout = bindings.timeout_for_prefix(&prefix);
            let policy = bindings.policy_for_prefix(&prefix);
            runtime::start_chord(origin, prefix, timeout, policy);
            return Some(Flow::Handled);
        }
    }

    bindings.dispatch(key)
}

fn dispatch_timeout(global_bindings: &Keys) -> Flow {
    let Some(pending) = runtime::pending_chord() else {
        return Flow::Ignored;
    };
    if runtime::chord_stale(pending.origin) {
        runtime::reset_chord();
        return Flow::Ignored;
    }

    let Some(expired) = runtime::take_expired_chord() else {
        return Flow::Ignored;
    };
    fallback_first(expired.origin, expired.first_key, global_bindings)
}

fn fallback_first(origin: runtime::ChordOrigin, first_key: Key, global_bindings: &Keys) -> Flow {
    bindings_for(origin, global_bindings)
        .and_then(|bindings| bindings.dispatch(first_key))
        .unwrap_or(Flow::Handled)
}

fn bindings_for(origin: runtime::ChordOrigin, global_bindings: &Keys) -> Option<Keys> {
    match origin {
        runtime::ChordOrigin::Focus(_)
        | runtime::ChordOrigin::View(_)
        | runtime::ChordOrigin::Layer(_) => runtime::keys_for(origin),
        runtime::ChordOrigin::Global => Some(global_bindings.clone()),
    }
}
