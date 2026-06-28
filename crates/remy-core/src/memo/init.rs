use crate::handle::Init;
use crate::app::App;

use super::Memo;

pub fn memo<T, F>(derive: F) -> MemoInit<F, T>
where
    F: Fn() -> T + Send + Sync + 'static,
    T: Send + Sync + Clone + PartialEq + 'static,
{
    MemoInit {
        derive,
        _phantom: std::marker::PhantomData,
    }
}

pub struct MemoInit<F, T> {
    pub derive: F,
    pub _phantom: std::marker::PhantomData<T>,
}

impl<F, T> Init<Memo<T>> for MemoInit<F, T>
where
    F: Fn() -> T + Send + Sync + 'static,
    T: Send + Sync + Clone + PartialEq + 'static,
{
    fn install(self, handle: &'static Memo<T>, _cx: App) {
        handle.install(self.derive);
    }
}
