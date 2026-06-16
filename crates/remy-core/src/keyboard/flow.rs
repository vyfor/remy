#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Ignored,
    Handled,
    Quit,
}

pub trait IntoFlow {
    fn into_key_result(self) -> Flow;
}

impl IntoFlow for () {
    fn into_key_result(self) -> Flow {
        Flow::Handled
    }
}

impl IntoFlow for Flow {
    fn into_key_result(self) -> Flow {
        self
    }
}

pub fn quit() -> Flow {
    Flow::Quit
}
